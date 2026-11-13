//! Version-three profiles pin the Cloud host device/key, never SDP or addresses.
use super::*;
impl ProfileManager {
    pub(super) async fn open_rtc_leased(
        &self,
        lease: directory::Lease,
    ) -> Result<RemoteClient, ClientError> {
        let mut profile = lease
            .profile
            .clone()
            .ok_or_else(|| fail("profile_not_found", "remote profile does not exist"))?;
        let (remote, metadata) = self.open_rtc_profile(&profile).await?;
        if profile.version == 2 {
            profile.version = 3;
            profile.peer = Some(metadata);
            profile.validate()?;
            let saved = profile.clone();
            if let Err(error) = tokio::task::spawn_blocking(move || lease.write(&saved))
                .await
                .map_err(|_| profile_error())?
            {
                remote.close();
                return Err(error);
            }
        }
        Ok(remote)
    }
    pub(super) async fn open_rtc_profile(
        &self,
        profile: &RemoteProfile,
    ) -> Result<(RemoteClient, PeerProfile), ClientError> {
        let context = self.cloud_context().await;
        let peer = profile.peer.as_ref().ok_or_else(protocol)?;
        let (mut remote, metadata) = context
            .dial_rtc(
                &peer.cloud_host_id,
                profile.client_kind,
                Some(peer),
                Some(&profile.host_id),
                Some(&profile.origin),
            )
            .await?;
        if remote.hello().principal_id != profile.principal_id
            || remote.hello().session_id != profile.session_id
        {
            remote.close();
            return Err(fail(
                "host_changed",
                "WebRTC session no longer matches the approved profile",
            ));
        }
        remote.profile_binding = Some((self.clone(), profile.name.clone()));
        Ok((remote, metadata))
    }
}
