use super::*;
use secrecy::ExposeSecret;
use zeroize::Zeroizing;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    profile_id: String,
    origin: String,
    host_id: String,
    principal_id: String,
    session_id: String,
    access_token: String,
    refresh_token: String,
    access_expires_at: i64,
    expires_at: i64,
    rotation_marker: Option<String>,
}
impl Drop for Record {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}
pub(super) fn encode(
    profile: &RemoteProfile,
    credentials: &NativeCredentials,
    rotation_marker: Option<String>,
) -> Result<SecretString, ClientError> {
    let record = Record {
        profile_id: profile.profile_id.clone(),
        origin: credentials.origin.clone(),
        host_id: credentials.host_id.clone(),
        principal_id: credentials.principal_id.clone(),
        session_id: credentials.session_id.clone(),
        access_token: credentials.access.expose_secret().into(),
        refresh_token: credentials.refresh.expose_secret().into(),
        access_expires_at: credentials.access_expires_at,
        expires_at: credentials.expires_at,
        rotation_marker,
    };
    let json = Zeroizing::new(serde_json::to_string(&record).map_err(|_| credential_error())?);
    Ok(SecretString::from(json.as_str().to_owned()))
}
pub(super) fn decode(
    profile: &RemoteProfile,
    secret: &SecretString,
) -> Result<(NativeCredentials, Option<String>), ClientError> {
    if secret.expose_secret().len() > 8192 {
        return Err(credential_error());
    }
    let record: Record =
        serde_json::from_str(secret.expose_secret()).map_err(|_| credential_error())?;
    if record.profile_id != profile.profile_id
        || record.origin != profile.origin
        || record.host_id != profile.host_id
        || record.principal_id != profile.principal_id
        || record.session_id != profile.session_id
        || record
            .rotation_marker
            .as_ref()
            .is_some_and(|id| !valid_id(id))
    {
        return Err(fail(
            "profile_invalid",
            "credential belongs to a different remote profile",
        ));
    }
    let valid = |s: &str| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if !valid(&record.access_token)
        || !valid(&record.refresh_token)
        || record.access_expires_at <= 0
        || record.expires_at <= 0
    {
        return Err(credential_error());
    }
    Ok((
        NativeCredentials {
            origin: record.origin.clone(),
            host_id: record.host_id.clone(),
            principal_id: record.principal_id.clone(),
            session_id: record.session_id.clone(),
            access: SecretString::from(record.access_token.clone()),
            refresh: SecretString::from(record.refresh_token.clone()),
            access_expires_at: record.access_expires_at,
            expires_at: record.expires_at,
        },
        record.rotation_marker.clone(),
    ))
}
