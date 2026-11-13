//! Trusted native WebRTC coordination. None of these hooks are remote RPC.
use super::*;
use crate::cloud_teams::PeerEndpointRecord;

impl CloudPeerHostBridge {
    fn rtc_source(&self, session: &cloud_client::Session) -> CloudPeerSource {
        CloudPeerSource {
            origin: session.instance_origin(),
            account_id: session.account_id().into(),
            host_id: self.host.clone(),
        }
    }
    pub fn rtc_source_matches(&self, expected: &CloudPeerSource) -> bool {
        if !matches!(self.teams.host_binding(),Ok(Some((host,device))) if host==self.host && device==self.device)
        {
            return false;
        }
        self.account
            .auth()
            .with_sync_identity_permit(|identity| {
                identity.is_some_and(|(origin, account)| {
                    origin == expected.origin
                        && account == expected.account_id
                        && self.host == expected.host_id
                })
            })
            .unwrap_or(false)
    }
    /// A missing registration authorizes initial key creation; an unreachable
    /// backend or an existing registration never authorizes key replacement.
    pub async fn rtc_registration(
        &self,
    ) -> Result<(CloudPeerSource, Option<PeerEndpointRecord>), OperationError> {
        let auth = self.account.auth();
        let captured = auth
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("sign in to Cloud first"))?;
        let source = self.rtc_source(&captured);
        let record = match self.teams.own_peer_endpoint(auth, &self.device).await {
            Ok(record) => Some(record),
            Err(error) if error.code == "not_found" => None,
            Err(error) => return Err(error),
        };
        if self.teams.host_binding()?.as_ref() != Some(&(self.host.clone(), self.device.clone())) {
            return Err(OperationError::conflict("Cloud host binding changed"));
        }
        let _permit = auth
            .try_session_emission_permit(&captured)
            .map_err(|_| OperationError::unavailable("Cloud account changed"))?;
        Ok((source, record))
    }
    pub async fn rtc_ice_config(
        &self,
        source: &CloudPeerSource,
    ) -> Result<crate::cloud_teams::RtcTransportConfig, OperationError> {
        let auth = self.account.auth();
        let captured = auth
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("sign in to Cloud first"))?;
        let current = self.rtc_source(&captured);
        if current.origin != source.origin
            || current.account_id != source.account_id
            || current.host_id != source.host_id
        {
            return Err(OperationError::unavailable("Cloud account changed"));
        }
        let config = self.teams.rtc_config(auth).await?;
        self.commit(captured, move |_| Ok(config)).await
    }
    pub async fn rtc_ticket(&self, source: &CloudPeerSource) -> Result<String, OperationError> {
        let auth = self.account.auth();
        let captured = auth
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("sign in to Cloud first"))?;
        let current = self.rtc_source(&captured);
        if current.origin != source.origin
            || current.account_id != source.account_id
            || current.host_id != source.host_id
        {
            return Err(OperationError::unavailable("Cloud account changed"));
        }
        let ticket = self
            .teams
            .issue_ticket(auth, &self.host, &self.device)
            .await?;
        if self.teams.host_binding()?.as_ref() != Some(&(self.host.clone(), self.device.clone())) {
            return Err(OperationError::conflict("Cloud host binding changed"));
        }
        let _permit = auth
            .try_session_emission_permit(&captured)
            .map_err(|_| OperationError::unavailable("Cloud account changed"))?;
        Ok(ticket)
    }
    /// Pin to an active confirmed grant from a fresh, complete owner snapshot,
    /// then require that same binding to exist in the local authority store.
    pub async fn rtc_peer_key(
        &self,
        source: CloudPeerSource,
        device: String,
        revision: i64,
    ) -> Result<[u8; 32], OperationError> {
        let auth = self.account.auth();
        let captured = auth
            .session()
            .await
            .ok_or_else(|| OperationError::unavailable("sign in to Cloud first"))?;
        let current = self.rtc_source(&captured);
        if current.origin != source.origin
            || current.account_id != source.account_id
            || current.host_id != source.host_id
        {
            return Err(OperationError::unavailable("Cloud account changed"));
        }
        let snapshot = self.teams.peer_authorizations(auth, &self.host).await?;
        let item = snapshot
            .items
            .iter()
            .find(|item| {
                item.device_id == device
                    && item.status == "active"
                    && item.device_status == "active"
                    && item.revision == revision
                    && item.confirmed_revision == Some(revision)
                    && policy::grant(item).is_ok()
            })
            .filter(|_| snapshot.host_status == "active")
            .ok_or_else(|| OperationError::forbidden("native grant is not confirmed"))?;
        let endpoint = item
            .endpoint_id
            .clone()
            .ok_or_else(|| OperationError::forbidden("device endpoint is unavailable"))?;
        self.commit(captured, move |bridge| {
            bridge.reconcile(&source, &snapshot)?;
            bridge
                .service
                .authenticate_rtc_peer(&endpoint, &source, &device, revision)
                .map_err(local)?;
            let mut key = [0; 32];
            if endpoint.len() != 64 {
                return Err(OperationError::invalid("invalid peer key"));
            }
            for (index, byte) in key.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&endpoint[index * 2..index * 2 + 2], 16)
                    .map_err(|_| OperationError::invalid("invalid peer key"))?;
            }
            Ok(key)
        })
        .await
    }
}
