use crate::{
    auth::{self, Authentication, HttpError},
    state::State,
};
use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        State as ExtractState,
    },
    http::HeaderMap,
    response::Response,
};
use serde_json::Value;
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tiktools_control_api::{ApiError, RemoteSession, RequestContext, RpcId, RpcResponse};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
pub(crate) mod wire;
use wire::Wire;

// A cancelled or timed-out blocking check may still be running. Keep its
// permit until the blocking job actually exits, across gateway restarts.
fn heartbeat_slots() -> Arc<Semaphore> {
    static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    Arc::clone(SLOTS.get_or_init(|| Arc::new(Semaphore::new(64))))
}

async fn heartbeat_check(
    stopping: &CancellationToken,
    slots: Arc<Semaphore>,
    timeout: Duration,
    operation: impl FnOnce() -> bool + Send + 'static,
) -> bool {
    tokio::select! {
        biased;
        _ = stopping.cancelled() => false,
        result = tokio::time::timeout(timeout, async move {
            let Ok(permit) = slots.acquire_owned().await else { return false };
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                operation()
            }).await.unwrap_or(false)
        }) => result.unwrap_or(false),
    }
}

pub(crate) async fn upgrade(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, HttpError> {
    let authentication = auth::authenticate(&state, &headers, true).await?;
    if authentication.browser {
        auth::consume_ticket(&state, &authentication, &headers)?;
    }
    let identity = authentication.authority.snapshot()?;
    let permit = state.admit(&identity.principal_id, &identity.session_id)?;
    Ok(ws
        .max_frame_size(state.configuration.limits.frame_bytes)
        .max_message_size(state.configuration.limits.frame_bytes)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            run(socket.into(), state, authentication).await;
        }))
}

pub(crate) async fn run(mut socket: Wire, state: Arc<State>, authentication: Authentication) {
    let max_frame = state.configuration.limits.frame_bytes;
    let session = match RemoteSession::from_authorization(authentication.authority.clone()) {
        Ok(session) => session.with_connection(
            uuid::Uuid::new_v4().to_string(),
            state.configuration.limits.clone(),
        ),
        Err(_) => return,
    };
    let context = RequestContext::Remote(session.clone());
    let connection = session
        .connection_id()
        .expect("gateway assigned connection ID")
        .to_owned();
    let _stream_cleanup = StreamCleanup {
        api: Arc::clone(&state.api),
        connection: connection.clone(),
    };
    let mut negotiated = false;
    let mut calls: JoinSet<(String, RpcResponse)> = JoinSet::new();
    let mut checks = tokio::time::interval(Duration::from_secs(2));
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    let mut stream_clock = tokio::time::interval(Duration::from_millis(25));
    stream_clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let hello_deadline = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(hello_deadline);
    let mut last_received = Instant::now();
    loop {
        let outgoing = tokio::select! {
            _=state.stopping.cancelled()=>break,
            _=&mut hello_deadline,if !negotiated=>break,
            _=checks.tick()=>{
                if session.grant().is_err()||last_received.elapsed()>=Duration::from_secs(30){break;}
                continue;
            },
            _=heartbeat.tick()=>{
                let service=Arc::clone(&state.service);let token=authentication.token.clone();let browser=authentication.browser;
                #[cfg(feature="peer")]
                let peer_id=authentication.peer_id.clone();
                if !heartbeat_check(&state.stopping, heartbeat_slots(), Duration::from_secs(5), move || {
                    #[cfg(feature="peer")]
                    if let Some(peer_id)=peer_id { return service.authenticate_peer(&peer_id).is_ok(); }
                    service.authenticate_live(&token,browser,true).is_ok()
                }).await {break;}
                if !matches!(tokio::time::timeout(Duration::from_secs(5),socket.send(Message::Ping(Vec::new().into()))).await,Ok(Ok(()))){break;}
                continue;
            },
            _=stream_clock.tick(),if negotiated=>{
                let batch_start=Instant::now();
                for _ in 0..8 {
                    let next=state.api.core().collaboration_stream_next(&authentication.authority,&connection);
                    let event=match next {
                        Ok(Some(event))=>event,
                        Ok(None)=>break,
                        Err(_)=>{
                            let _=send(&mut socket,&serde_json::json!({"jsonrpc":"2.0","method":"event.gap","params":{"code":"resync_required"}}),max_frame).await;
                            return;
                        }
                    };
                    if state.api.core().collaboration_stream_event_authorize(&authentication.authority,&connection,&event).is_err() || send(&mut socket,&serde_json::json!({"jsonrpc":"2.0","method":"event","params":event}),max_frame).await.is_err()
                        || state.api.core().collaboration_stream_sent(&authentication.authority,&connection,&event).is_err() { return; }
                    if batch_start.elapsed()>=Duration::from_millis(25) {break;}
                }
                continue;
            },
            message=socket.recv()=>{
                let raw=match message{
                    Some(Ok(Message::Text(raw)))=>{last_received=Instant::now();raw},
                    Some(Ok(Message::Ping(bytes)))=>{last_received=Instant::now();if !matches!(tokio::time::timeout(Duration::from_secs(5),socket.send(Message::Pong(bytes))).await,Ok(Ok(()))){break;}continue;},
                    Some(Ok(Message::Pong(_)))=>{last_received=Instant::now();continue;},
                    _=>break,
                };
                let value:Value=match serde_json::from_str(&raw){Ok(value)=>value,Err(_)=>break};
                let id=RpcId::extract(&value);
                let valid_id=value.get("id").is_some_and(|id|id.as_i64().is_some()||id.as_str().is_some_and(|id|!id.is_empty()&&id.len()<=128));
                let method=value.get("method").and_then(Value::as_str).unwrap_or("").to_owned();
                if state.rate(session.session_id()).is_err(){
                    let mut error=ApiError::new("rate_limited","remote session request rate exceeded");error.data=Some(serde_json::json!({"retryAfterMs":1000}));
                    RpcResponse::error(id,error)
                }else if !valid_id||value.get("jsonrpc").and_then(Value::as_str)!=Some("2.0")||method.is_empty()||method.len()>128{
                    RpcResponse::error(id,ApiError::new("invalid_request","a JSON-RPC 2.0 request with an id is required"))
                }else if value.get("params").is_some_and(|params|serde_json::to_vec(params).map_or(true,|bytes|bytes.len()>state.configuration.limits.params_bytes)){
                    RpcResponse::error(id,ApiError::too_large())
                }else if !negotiated{
                    if method!="collaboration.hello"{
                        let response=RpcResponse::error(id,ApiError::new("protocol_incompatible","negotiate collaboration.hello first"));
                        let _=send(&mut socket,&response,max_frame).await;break;
                    }
                    let mut response=state.api.execute_value_with_context(&context,&value).await;
                    if let Some(features)=response.result.as_mut().and_then(|value|value.get_mut("features")).and_then(serde_json::Value::as_object_mut) {
                        features.insert("browserApp".into(),serde_json::Value::Bool(crate::http::browser_app_enabled(&state)));
                    }
                    if response.error.as_ref().is_some_and(|error|error.code=="protocol_incompatible"){
                        let _=send(&mut socket,&response,max_frame).await;break;
                    }
                    negotiated=response.error.is_none();response
                }else if matches!(method.as_str(),"collaboration.subscribe"|"collaboration.snapshot"|"collaboration.resume") && calls.len()<state.configuration.limits.pending_calls as usize {
                    // Recovery operations have connection-local ordering. Send
                    // their acknowledgement before releasing notifications.
                    let response=state.api.execute_value_with_context(&context,&value).await;
                    match response.result.as_ref().map_or_else(||state.api.authorize_remote_failure(&session,&method,response.error.as_ref().and_then(|e|e.data.as_ref())),|result|state.api.authorize_remote_response(&session,&method,result)) {
                        Ok(())=>response,Err(error)=>RpcResponse::error(response.id,error)
                    }
                }else if calls.len()>=state.configuration.limits.pending_calls as usize{
                    RpcResponse::error(id,ApiError::new("overloaded","remote pending-call limit exceeded"))
                }else{
                    let api=Arc::clone(&state.api);let context=context.clone();
                    calls.spawn(async move{let result=api.execute_value_with_context(&context,&value).await;(method,result)});
                    continue;
                }
            },
            result=calls.join_next(),if !calls.is_empty()=>{
                match result{
                    Some(Ok((method,response)))=>match response.result.as_ref().map_or_else(||state.api.authorize_remote_failure(&session,&method,response.error.as_ref().and_then(|error|error.data.as_ref())),|result|state.api.authorize_remote_response(&session,&method,result)){
                        Ok(())=>response,Err(error)=>RpcResponse::error(response.id,error),
                    },
                    _=>break,
                }
            }
        };
        if session.grant().is_err() || send(&mut socket, &outgoing, max_frame).await.is_err() {
            break;
        }
    }
    calls.abort_all();
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.send(Message::Close(None))).await;
}

struct StreamCleanup {
    api: Arc<tiktools_control_api::ControlApi>,
    connection: String,
}
impl Drop for StreamCleanup {
    fn drop(&mut self) {
        self.api
            .core()
            .collaboration_stream_detach(&self.connection);
    }
}

async fn send(
    socket: &mut Wire,
    response: &impl serde::Serialize,
    max_frame: usize,
) -> Result<(), ()> {
    let mut writer = LimitedWriter(Vec::with_capacity(4096), max_frame);
    serde_json::to_writer(&mut writer, response).map_err(|_| ())?;
    let text = String::from_utf8(writer.0).map_err(|_| ())?;
    match tokio::time::timeout(
        Duration::from_secs(5),
        socket.send(Message::Text(text.into())),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        _ => Err(()),
    }
}

struct LimitedWriter(Vec<u8>, usize);
impl std::io::Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > self.1 {
            return Err(std::io::Error::other("remote response exceeds frame limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_FRAME;
    use serde_json::json;
    use std::sync::mpsc;
    #[test]
    fn output_is_bounded_before_json_frame_allocation() {
        let response =
            RpcResponse::success(RpcId::Number(1), json!({"payload":"x".repeat(MAX_FRAME+1)}));
        let mut output = LimitedWriter(Vec::new(), MAX_FRAME);
        assert!(serde_json::to_writer(&mut output, &response).is_err());
        assert!(output.0.len() <= MAX_FRAME);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn heartbeat_shutdown_is_prompt_and_stalled_worker_keeps_its_slot() {
        let stopping = CancellationToken::new();
        let slots = Arc::new(Semaphore::new(1));
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let task = {
            let stopping = stopping.clone();
            let slots = Arc::clone(&slots);
            tokio::spawn(async move {
                heartbeat_check(&stopping, slots, Duration::from_secs(30), move || {
                    let _ = ready_tx.send(());
                    release_rx.recv().is_ok()
                })
                .await
            })
        };
        tokio::time::timeout(Duration::from_secs(2), ready_rx)
            .await
            .unwrap()
            .unwrap();
        stopping.cancel();
        assert!(!tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap());
        assert!(slots.clone().try_acquire_owned().is_err());
        release_tx.send(()).unwrap();
        let permit = tokio::time::timeout(Duration::from_secs(2), slots.acquire_owned())
            .await
            .unwrap()
            .unwrap();
        drop(permit);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn heartbeat_timeout_bounds_stalled_checks_and_recovers_capacity() {
        let stopping = CancellationToken::new();
        let slots = Arc::new(Semaphore::new(1));
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let slow = {
            let stopping = stopping.clone();
            let slots = Arc::clone(&slots);
            tokio::spawn(async move {
                heartbeat_check(&stopping, slots, Duration::from_millis(200), move || {
                    let _ = ready_tx.send(());
                    release_rx.recv().is_ok()
                })
                .await
            })
        };
        tokio::time::timeout(Duration::from_secs(2), ready_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(!slow.await.unwrap());
        assert!(
            !heartbeat_check(
                &stopping,
                Arc::clone(&slots),
                Duration::from_millis(50),
                || true
            )
            .await
        );
        release_tx.send(()).unwrap();
        let permit = tokio::time::timeout(Duration::from_secs(2), slots.clone().acquire_owned())
            .await
            .unwrap()
            .unwrap();
        drop(permit);
        assert!(heartbeat_check(&stopping, slots, Duration::from_secs(1), || true).await);
    }
}
