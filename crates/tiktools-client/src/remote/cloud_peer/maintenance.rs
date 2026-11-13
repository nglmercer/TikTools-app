//! Refresh device publication independently of RPC traffic. An outage never
//! closes the data connection or extends the host's local authorization lease.
use super::*;
use tokio_util::sync::CancellationToken;

const REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct MaintenanceContext {
    pub(super) auth: Arc<CloudAuth>,
    pub(super) data: PathBuf,
}

impl ProfileManager {
    pub(crate) fn set_cloud_context(&self, context: MaintenanceContext) {
        *self.cloud_context.lock().unwrap_or_else(|p| p.into_inner()) = Some(context);
    }

    pub(crate) async fn cloud_context(&self) -> MaintenanceContext {
        let existing = self
            .cloud_context
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if let Some(context) = existing {
            // Desktop uses the live account, including memory-only login.
            // Never restore a saved account behind an explicit logout.
            return context;
        }
        let data = self.directory().join("cloud-account");
        let context = MaintenanceContext::new(
            Arc::new(CloudAuth::from_environment(
                data.clone(),
                Arc::new(tiktools_cloud_auth::OsTokenStore::new()),
            )),
            data,
        );
        // A profile-only CLI process may restore a saved login without starting
        // an interactive login. Opening a new WebRTC channel requires Cloud;
        // existing channels keep working independently during an outage.
        let _ = tokio::time::timeout(REFRESH_TIMEOUT, context.auth.restore()).await;
        let mut slot = self.cloud_context.lock().unwrap_or_else(|p| p.into_inner());
        slot.get_or_insert(context).clone()
    }
}

impl MaintenanceContext {
    pub(crate) fn new(auth: Arc<CloudAuth>, data: PathBuf) -> Self {
        Self { auth, data }
    }

    pub(super) fn binding(
        &self,
        peer: &PeerProfile,
        origin: &str,
        account: &str,
        endpoint: &str,
    ) -> Result<(CloudTeams, String), ClientError> {
        let (teams, data) = scoped(&self.data, origin, account);
        if peer.key_path != data.join("identity.key") || endpoint != peer.local_endpoint_id {
            return Err(fail(
                "account_changed",
                "profile does not belong to this Cloud identity",
            ));
        }
        let device = teams.controller_device().map_err(cloud)?;
        Ok((teams, device))
    }
}

pub(super) async fn maintain<V, F, Fut>(stop: CancellationToken, mut valid: V, mut refresh: F)
where
    V: FnMut() -> bool,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), ClientError>>,
{
    let mut tick = tokio::time::interval(REFRESH_INTERVAL);
    tick.tick().await;
    let mut fence = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            biased;
            _=stop.cancelled()=>return,
            _=fence.tick()=>{if !valid(){stop.cancel();return;}},
            _=tick.tick()=>{
                let request=tokio::time::timeout(REFRESH_TIMEOUT,refresh());tokio::pin!(request);
                loop {tokio::select! {
                    biased;
                    _=stop.cancelled()=>return,
                    _=fence.tick()=>{if !valid(){stop.cancel();return;}},
                    _=&mut request=>break,
                }}
            },
        }
    }
}
#[cfg(test)]
mod tests;
