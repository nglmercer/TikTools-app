//! Native Cloud management. No bearer/key material crosses the WebView bridge.
use super::*;
mod network;
#[cfg(test)]
mod tests;
pub(super) use network::{cancel as cancel_network, NetworkState};
use tiktools_cloud_auth::AuthError;
use zeroize::Zeroizing;

pub(super) struct LoginJob {
    operation: String,
    task: tokio::task::JoinHandle<()>,
}
pub(super) fn stop_login(slot: &std::sync::Mutex<Option<LoginJob>>, operation: Option<&str>) {
    if let Ok(mut slot) = slot.lock() {
        if slot
            .as_ref()
            .is_some_and(|job| operation.is_none_or(|operation| operation == job.operation))
        {
            if let Some(job) = slot.take() {
                job.task.abort();
            }
        }
    }
}
fn auth(error: AuthError) -> ClientError {
    fail(error.code(), &error.to_string())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LoginOperation {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
fn parse<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, ClientError> {
    serde_json::from_value(params)
        .map_err(|_| fail("invalid_params", "invalid Cloud controller parameters"))
}

impl Controller {
    pub(super) async fn manage_cloud(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, ClientError> {
        let cloud = self
            .cloud
            .as_ref()
            .ok_or_else(|| {
                fail(
                    "unsupported_remote",
                    "Cloud P2P onboarding is unavailable in this controller",
                )
            })?
            .clone();
        match method {
            "controller.cloud.status" => {
                let _: Empty = parse(params)?;
                Ok(
                    json!({"account":cloud.account().status().await,"deviceId":cloud.controller_device_id().await.ok()}),
                )
            }
            "controller.cloud.restore" => {
                let _: Empty = parse(params)?;
                match cloud.account().restore().await {
                    Ok(status) => Ok(
                        json!({"account":status,"deviceId":cloud.controller_device_id().await.ok()}),
                    ),
                    Err(AuthError::NoSavedSession | AuthError::Expired) => {
                        Ok(json!({"account":cloud.account().status().await,"deviceId":null}))
                    }
                    Err(error) => Err(auth(error)),
                }
            }
            "controller.cloud.login.begin" => {
                let _: Empty = parse(params)?;
                let _lifecycle = self.lifecycle.lock().await;
                if !matches!(self.status().phase.as_str(), "idle" | "failed") {
                    return Err(fail(
                        "invalid_state",
                        "disconnect before signing in to Cloud",
                    ));
                }
                let operation = cloud.account().reserve_login().await.map_err(auth)?;
                let begin = cloud
                    .account()
                    .begin_login_for(Some(&operation))
                    .await
                    .map_err(auth)?;
                let login = cloud.clone();
                let login_operation = operation.clone();
                let shared = self.shared.clone();
                let task = tokio::spawn(async move {
                    let _ = login
                        .account()
                        .complete_login_for(Some(&login_operation))
                        .await;
                    shared
                        .changed
                        .send_modify(|revision| *revision = revision.wrapping_add(1));
                });
                let mut slot = self.cloud_login.lock().map_err(|_| {
                    task.abort();
                    fail("internal", "Cloud login task lock unavailable")
                })?;
                if let Some(previous) = slot.replace(LoginJob {
                    operation: operation.clone(),
                    task,
                }) {
                    previous.task.abort();
                }
                Ok(json!({"operationId":operation,"browserUrl":begin.browser_url}))
            }
            "controller.cloud.login.open" => {
                let p: LoginOperation = parse(params)?;
                cloud
                    .account()
                    .open_browser_for(Some(&p.operation_id))
                    .await
                    .map_err(auth)?;
                Ok(json!({"ok":true}))
            }
            "controller.cloud.login.cancel" => {
                let p: LoginOperation = parse(params)?;
                stop_login(&self.cloud_login, Some(&p.operation_id));
                let account = cloud
                    .account()
                    .cancel_login_for(Some(&p.operation_id))
                    .await
                    .map_err(auth)?;
                Ok(json!({"account":account,"deviceId":cloud.controller_device_id().await.ok()}))
            }
            "controller.cloud.logout" => {
                let _: Empty = parse(params)?;
                self.disconnect().await;
                stop_login(&self.cloud_login, None);
                let account = cloud.account().logout().await.map_err(auth)?;
                Ok(json!({"account":account,"deviceId":null}))
            }
            "controller.cloud.teams" => {
                let _: Empty = parse(params)?;
                Ok(json!({"items":cloud.teams().await?}))
            }
            "controller.cloud.join" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Join {
                    profile: String,
                    label: String,
                    token: String,
                }
                let mut p: Join = parse(params)?;
                let token = Zeroizing::new(std::mem::take(&mut p.token));
                let generation = network::generation(&self.cloud_network)?;
                let mut lifecycle = self.lifecycle.lock().await;
                let operation = network::begin(&self.cloud_network, generation)?;
                if !matches!(self.status().phase.as_str(), "idle" | "failed") {
                    return Err(fail(
                        "invalid_state",
                        "disconnect before joining another team",
                    ));
                }
                self.profiles.ensure_new(&p.profile).await?;
                let request = tokio::select! {
                    _=operation.token.cancelled()=>return Err(fail("cancelled","Cloud connection cancelled")),
                    request=async {
                match cloud.controller_device_id().await {
                    Ok(_) => {}
                    Err(error) if error.code == "invalid_params" => {
                        cloud.enroll(p.label).await?;
                    }
                    Err(error) => return Err(error),
                }
                cloud.redeem(token.to_string()).await
                    }=>request?,
                };
                let pending = CloudPairing {
                    profile: p.profile,
                    host_id: request.host_id,
                    request_id: request.request_id,
                };
                let epoch = self.reset(&mut lifecycle);
                lifecycle.cloud_pairing = Some(pending.clone());
                self.shared.change(&epoch, |view| {
                    view.status.phase = "cloud-pairing".into();
                    view.status.cloud_pairing = Some(pending.clone());
                });
                serde_json::to_value(pending).map_err(|_| protocol())
            }
            "controller.cloud.pair.poll" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Poll {
                    epoch: String,
                }
                let p: Poll = parse(params)?;
                let generation = network::generation(&self.cloud_network)?;
                let mut lifecycle = self.lifecycle.lock().await;
                let operation = network::begin(&self.cloud_network, generation)?;
                if self.status().epoch != p.epoch {
                    return Err(fail("stale_selection", "Cloud pairing selection changed"));
                }
                let pending = lifecycle
                    .cloud_pairing
                    .clone()
                    .ok_or_else(|| fail("invalid_state", "no Cloud pairing is pending"))?;
                let result = tokio::select! {
                    _=operation.token.cancelled()=>return Err(fail("cancelled","Cloud connection cancelled")),
                    result=cloud.connect(&pending.profile,&pending.host_id)=>result,
                };
                match result {
                    Ok(_) => {
                        lifecycle.cloud_pairing = None;
                        self.shared.change(&p.epoch, |view| {
                            view.status.phase = "idle".into();
                            view.status.cloud_pairing = None;
                        });
                        Ok(json!({"approved":true,"profile":pending.profile}))
                    }
                    Err(error)
                        if matches!(
                            error.code.as_str(),
                            "approval_pending"
                                | "transport"
                                | "unavailable"
                                | "rate_limited"
                                | "unauthorized"
                        ) =>
                    {
                        Ok(json!({"approved":false,"errorCode":error.code}))
                    }
                    Err(error) => Err(error),
                }
            }
            "controller.cloud.profile.create" => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Connect {
                    profile: String,
                    host_id: String,
                }
                let p: Connect = parse(params)?;
                let generation = network::generation(&self.cloud_network)?;
                let _lifecycle = self.lifecycle.lock().await;
                let operation = network::begin(&self.cloud_network, generation)?;
                if !matches!(self.status().phase.as_str(), "idle" | "failed") {
                    return Err(fail(
                        "invalid_state",
                        "disconnect before connecting another Cloud team",
                    ));
                }
                let saved = tokio::select! {
                    _=operation.token.cancelled()=>return Err(fail("cancelled","Cloud connection cancelled")),
                    result=cloud.connect(&p.profile,&p.host_id)=>result?,
                };
                Ok(json!({"profile":saved.name}))
            }
            _ => Err(fail(
                "method_not_found",
                "unknown Cloud controller operation",
            )),
        }
    }
}
