use super::*;
use crate::{CallResult, ControlEvent, RequestMetadata, ResponseMetadata};
use futures_util::{SinkExt, StreamExt};
use secrecy::ExposeSecret;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tiktools_control_api::{
    modules::collaboration::HelloResult, RpcId, RpcRequest, REQUEST_TIMEOUT,
};
use tokio::time::Instant;
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc, oneshot, OwnedSemaphorePermit, Semaphore},
};
use tokio_tungstenite::{
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig, Message},
    Connector, MaybeTlsStream, WebSocketStream,
};
use tokio_util::sync::CancellationToken;
#[cfg(test)]
mod tests;
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Answer = Result<CallResult<Value>, ClientError>;
enum HelloReply {
    Retry(Duration),
    Negotiated(HelloResult),
}

fn classify_hello(value: Value, expected_id: &str, retries: u8) -> Result<HelloReply, ClientError> {
    if value["jsonrpc"] != "2.0"
        || value["id"].as_str() != Some(expected_id)
        || value.get("error").is_some() && value.get("result").is_some()
    {
        return Err(protocol());
    }
    if value.pointer("/error/code").and_then(Value::as_str) == Some("rate_limited") {
        if retries >= 4 {
            return Err(fail(
                "rate_limited",
                "remote negotiation rate limit exceeded",
            ));
        }
        let wait = value
            .pointer("/error/data/retryAfterMs")
            .and_then(Value::as_u64)
            .unwrap_or(1000)
            .clamp(1000, 2000);
        return Ok(HelloReply::Retry(Duration::from_millis(wait)));
    }
    if value.get("error").is_some() {
        return Err(fail(
            "protocol_incompatible",
            "remote protocol negotiation was rejected",
        ));
    }
    let hello =
        serde_json::from_value::<HelloResult>(value.get("result").cloned().ok_or_else(protocol)?)
            .map_err(|_| protocol())?;
    Ok(HelloReply::Negotiated(hello))
}
struct Outbound {
    id: i64,
    text: String,
    answer: oneshot::Sender<Answer>,
    deadline: Instant,
    _permit: OwnedSemaphorePermit,
}
struct Pending {
    answer: oneshot::Sender<Answer>,
    deadline: Instant,
    _permit: OwnedSemaphorePermit,
}
struct Lifetime {
    stop: CancellationToken,
}
#[derive(Default)]
struct Diagnostics {
    resync: AtomicBool,
    error: Mutex<Option<ClientError>>,
}
impl Drop for Lifetime {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

/// One negotiated connection. Calls are sent once; reconnect/refresh is an
/// explicit higher-level operation and never retries an ambiguous command.
#[derive(Clone)]
pub struct RemoteClient {
    _lifetime: Arc<Lifetime>,
    outbound: mpsc::Sender<Outbound>,
    admission: Arc<Semaphore>,
    next_id: Arc<AtomicI64>,
    events: broadcast::Sender<ControlEvent>,
    hello: Arc<HelloResult>,
    stop: CancellationToken,
    diagnostics: Arc<Diagnostics>,
    #[cfg(feature = "remote-profiles")]
    pub(super) profile_binding: Option<(super::profiles::ProfileManager, String)>,
}
impl RemoteClient {
    pub async fn connect(
        endpoint: &RemoteEndpoint,
        credentials: &NativeCredentials,
        kind: ClientKind,
    ) -> Result<Self, ClientError> {
        if kind == ClientKind::Browser {
            return Err(fail("invalid_params", "native client kind is required"));
        }
        endpoint.verify_host(credentials).await?;
        if credentials.access_expires_at <= now() {
            return Err(fail("unauthorized", "remote access credential expired"));
        }
        let url = format!(
            "{}/remote/v1/ws",
            endpoint.origin.replacen("https://", "wss://", 1)
        );
        let mut request = url.into_client_request().map_err(|_| protocol())?;
        request.headers_mut().insert(
            "authorization",
            super::http::bearer(credentials.access.expose_secret())?,
        );
        let config = WebSocketConfig::default()
            .max_frame_size(Some(1024 * 1024))
            .max_message_size(Some(1024 * 1024))
            .write_buffer_size(4096)
            .max_write_buffer_size(2 * 1024 * 1024);
        let connected = tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async_tls_with_config(
                request,
                Some(config),
                true,
                Some(Connector::Rustls(endpoint.tls.clone())),
            ),
        )
        .await
        .map_err(|_| network())?;
        let (mut socket, _) = connected.map_err(|error| match error {
            tokio_tungstenite::tungstenite::Error::Http(ref response)
                if response.status().as_u16() == 401 =>
            {
                fail("unauthorized", "remote access credential was rejected")
            }
            tokio_tungstenite::tungstenite::Error::Http(ref response)
                if response.status().as_u16() == 429 =>
            {
                fail("rate_limited", "remote connection rate limit exceeded")
            }
            _ => network(),
        })?;
        let hello = tokio::time::timeout(Duration::from_secs(10), async {
            let mut retries = 0;
            let make_request = |id: &str| json!({"jsonrpc":"2.0","id":id,"method":"collaboration.hello","params":{"protocolVersion":1,"clientVersion":env!("CARGO_PKG_VERSION"),"clientKind":kind}});
            let mut hello_id = String::from("hello-0");
            send(&mut socket, Message::Text(make_request(&hello_id).to_string().into())).await?;
            loop {
                match socket.next().await {
                    Some(Ok(Message::Text(raw))) => {
                        let value: Value = serde_json::from_str(&raw).map_err(|_| protocol())?;
                        match classify_hello(value, &hello_id, retries)? {
                            HelloReply::Negotiated(hello) => return Ok(hello),
                            HelloReply::Retry(wait) => {
                                // Only an explicit refusal to negotiate is
                                // retried; mutations never use this path.
                                retries += 1;
                                tokio::time::sleep(wait).await;
                                hello_id = format!("hello-{retries}");
                                send(&mut socket, Message::Text(make_request(&hello_id).to_string().into())).await?;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        send(&mut socket, Message::Pong(bytes)).await?
                    }
                    _ => return Err(network()),
                }
            }
        })
        .await
        .map_err(|_| network())??;
        hello.limits.validate().map_err(|_| protocol())?;
        if hello.protocol_version != 1
            || hello.host_id != credentials.host_id
            || hello.principal_id != credentials.principal_id
            || hello.session_id != credentials.session_id
            || !valid_id(&hello.host_instance_id)
            || !valid_id(&hello.connection_id)
            || tiktools_collaboration::stream_sequence(&hello.grant_revision)
                .map_or(true, |n| n == 0)
        {
            return Err(fail(
                "host_changed",
                "negotiated host or session identity does not match the profile",
            ));
        }
        // The codec has an absolute 1-MiB cap. Enforce any negotiated lower
        // message bound before JSON parsing; never rebuild/drop its read buffer.
        Ok(Self::from_negotiated(socket, hello, credentials))
    }
    fn from_negotiated(
        socket: Socket,
        hello: HelloResult,
        credentials: &NativeCredentials,
    ) -> Self {
        let limit = hello.limits.pending_calls as usize;
        let (outbound, receiver) = mpsc::channel(limit);
        let (events, _) = broadcast::channel(256);
        let stop = CancellationToken::new();
        let hello = Arc::new(hello);
        let diagnostics = Arc::new(Diagnostics::default());
        tokio::spawn(run(
            socket,
            receiver,
            events.clone(),
            stop.clone(),
            hello.clone(),
            credentials.clone(),
            diagnostics.clone(),
        ));
        Self {
            _lifetime: Arc::new(Lifetime { stop: stop.clone() }),
            outbound,
            admission: Arc::new(Semaphore::new(limit)),
            next_id: Arc::new(AtomicI64::new(2)),
            events,
            hello,
            stop,
            diagnostics,
            #[cfg(feature = "remote-profiles")]
            profile_binding: None,
        }
    }
    pub fn hello(&self) -> &HelloResult {
        &self.hello
    }
    pub fn is_closed(&self) -> bool {
        self.stop.is_cancelled()
    }
    pub fn close(&self) {
        self.stop.cancel();
    }
    pub fn requires_resync(&self) -> bool {
        self.diagnostics.resync.load(Ordering::SeqCst)
    }
    pub fn closed_reason(&self) -> Option<ClientError> {
        self.diagnostics.error.lock().ok().and_then(|e| e.clone())
    }
    #[cfg(feature = "remote-profiles")]
    pub async fn reopen(&self) -> Result<Self, ClientError> {
        let (manager, name) = self.profile_binding.as_ref().ok_or_else(|| {
            fail(
                "profile_required",
                "reopening requires an explicitly selected remote profile",
            )
        })?;
        manager.open(name).await
    }
    pub fn subscribe(&self) -> broadcast::Receiver<ControlEvent> {
        self.events.subscribe()
    }
    pub async fn call_value_with_meta<P: Serialize>(
        &self,
        method: &str,
        params: P,
        meta: Option<RequestMetadata>,
    ) -> Result<CallResult<Value>, ClientError> {
        if self.is_closed() {
            return Err(network());
        }
        if method.is_empty() || method.len() > 128 || method.chars().any(char::is_control) {
            return Err(fail("invalid_params", "invalid remote method"));
        }
        let permit = self
            .admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| fail("overloaded", "remote pending-call limit exceeded"))?;
        let mut body = LimitedBuffer(Vec::with_capacity(1024), self.hello.limits.params_bytes);
        serde_json::to_writer(&mut body, &params).map_err(|_| {
            fail(
                "too_large",
                "remote parameters exceed the negotiated budget",
            )
        })?;
        let params: Value = serde_json::from_slice(&body.0).map_err(|_| protocol())?;
        let id = loop {
            let id = self.next_id.load(Ordering::SeqCst);
            let next = id.checked_add(1).ok_or_else(protocol)?;
            if self
                .next_id
                .compare_exchange_weak(id, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break id;
            }
        };
        let mut request = RpcRequest::new(RpcId::Number(id), method, params);
        request.meta = meta;
        let mut body = LimitedBuffer(Vec::with_capacity(1024), self.hello.limits.frame_bytes);
        serde_json::to_writer(&mut body, &request)
            .map_err(|_| fail("too_large", "remote request exceeds the negotiated budget"))?;
        let text = String::from_utf8(body.0).map_err(|_| protocol())?;
        let (answer, receiver) = oneshot::channel();
        self.outbound
            .try_send(Outbound {
                id,
                text,
                answer,
                deadline: Instant::now() + REQUEST_TIMEOUT,
                _permit: permit,
            })
            .map_err(|_| network())?;
        match tokio::time::timeout(REQUEST_TIMEOUT, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(network()),
            Err(_) => Err(fail(
                "timeout",
                "remote request timed out; resolve its command ID before retrying",
            )),
        }
    }
}
async fn send(socket: &mut Socket, message: Message) -> Result<(), ClientError> {
    tokio::time::timeout(Duration::from_secs(5), socket.send(message))
        .await
        .map_err(|_| network())?
        .map_err(|_| network())
}
async fn run(
    mut socket: Socket,
    mut outbound: mpsc::Receiver<Outbound>,
    events: broadcast::Sender<ControlEvent>,
    stop: CancellationToken,
    hello: Arc<HelloResult>,
    credentials: NativeCredentials,
    diagnostics: Arc<Diagnostics>,
) {
    let mut pending: BTreeMap<i64, Pending> = BTreeMap::new();
    let mut retired = VecDeque::new();
    let mut last_received = Instant::now();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    let mut checks = tokio::time::interval(Duration::from_millis(100));
    checks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let error = loop {
        let result: Result<(), ClientError> = tokio::select! {
            _=stop.cancelled()=>break network(),
            _=heartbeat.tick()=>{send(&mut socket,Message::Ping(Vec::new().into())).await},
            _=checks.tick()=>{
                if last_received.elapsed()>=Duration::from_secs(30){break fail("transport","remote heartbeat expired");}
                if credentials.access_expires_at<=now(){break fail("unauthorized","remote access credential expired; reopen the profile to refresh");}
                let expired:Vec<_>=pending.iter().filter(|(_,p)|p.deadline<=Instant::now()).map(|(id,_)|*id).collect();
                for id in expired {if let Some(p)=pending.remove(&id){let _=p.answer.send(Err(fail("timeout","remote request timed out; resolve its command ID before retrying")));retire(&mut retired,id);}}
                Ok(())
            },
            request=outbound.recv()=>{
                let Some(request)=request else {break network();};
                if request.answer.is_closed()||request.deadline<=Instant::now(){continue;}
                match send(&mut socket,Message::Text(request.text.into())).await {
                    Ok(())=>{pending.insert(request.id,Pending{answer:request.answer,deadline:request.deadline,_permit:request._permit});Ok(())},
                    Err(error)=>{let _=request.answer.send(Err(error.clone()));Err(error)}
                }
            },
            frame=socket.next()=>{
                match frame {
                    Some(Ok(Message::Text(raw)))=>{last_received=Instant::now();if raw.len()>hello.limits.frame_bytes {Err(protocol())} else {process(&raw,&hello,&credentials,&mut pending,&mut retired,&events,&diagnostics)}},
                    Some(Ok(Message::Ping(bytes)))=>{last_received=Instant::now();send(&mut socket,Message::Pong(bytes)).await},
                    Some(Ok(Message::Pong(_)))=>{last_received=Instant::now();Ok(())},
                    _=>Err(network())
                }
            }
        };
        if let Err(error) = result {
            break error;
        }
    };
    if error.code == "protocol" {
        diagnostics.resync.store(true, Ordering::SeqCst);
    }
    if let Ok(mut stored) = diagnostics.error.lock() {
        *stored = Some(error.clone());
    }
    stop.cancel();
    let _ = events.send(ControlEvent::Gap { lost: 0 });
    for (_, p) in pending {
        let _ = p.answer.send(Err(error.clone()));
    }
    while let Ok(request) = outbound.try_recv() {
        let _ = request.answer.send(Err(error.clone()));
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.send(Message::Close(None))).await;
}
fn retire(retired: &mut VecDeque<i64>, id: i64) {
    retired.push_back(id);
    if retired.len() > 256 {
        retired.pop_front();
    }
}
fn process(
    raw: &str,
    hello: &HelloResult,
    credentials: &NativeCredentials,
    pending: &mut BTreeMap<i64, Pending>,
    retired: &mut VecDeque<i64>,
    events: &broadcast::Sender<ControlEvent>,
    diagnostics: &Diagnostics,
) -> Result<(), ClientError> {
    let mut value: Value = serde_json::from_str(raw).map_err(|_| protocol())?;
    if value["jsonrpc"] != "2.0" {
        return Err(protocol());
    }
    if value["method"] == "event.gap" {
        diagnostics.resync.store(true, Ordering::SeqCst);
        let _ = events.send(ControlEvent::Gap {
            lost: value
                .pointer("/params/lost")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        });
        return Ok(());
    }
    if value["method"] == "event" {
        let event: ReliableEvent =
            serde_json::from_value(value.get("params").cloned().ok_or_else(protocol)?)
                .map_err(|_| protocol())?;
        if event.host_instance_id != hello.host_instance_id
            || tiktools_collaboration::stream_sequence(&event.sequence).map_or(true, |n| n == 0)
            || !valid_id(&event.subscription_id)
        {
            return Err(protocol());
        }
        let _ = events.send(ControlEvent::Reliable(event));
        return Ok(());
    }
    if value.get("method").is_some() {
        return Err(protocol());
    }
    let id = value["id"].as_i64().ok_or_else(protocol)?;
    if !pending.contains_key(&id) {
        return if retired.contains(&id) {
            Ok(())
        } else {
            Err(protocol())
        };
    }
    let outcome = if value.get("error").is_some() {
        if value.get("result").is_some() {
            return Err(protocol());
        }
        credentials.redact(&mut value);
        let error = value.get("error").ok_or_else(protocol)?;
        let code = error
            .get("code")
            .and_then(Value::as_str)
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 64
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            })
            .ok_or_else(protocol)?;
        Err(ClientError {
            code: code.into(),
            message: "remote host rejected the operation".into(),
            data: error.get("data").cloned(),
        })
    } else {
        let result = value.get("result").cloned().ok_or_else(protocol)?;
        let meta = value
            .get("meta")
            .cloned()
            .map(serde_json::from_value::<ResponseMetadata>)
            .transpose()
            .map_err(|_| protocol())?;
        Ok(CallResult { result, meta })
    };
    let entry = pending.remove(&id).ok_or_else(protocol)?;
    let _ = entry.answer.send(outcome);
    retire(retired, id);
    Ok(())
}
