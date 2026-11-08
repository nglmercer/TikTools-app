use super::*;
use secrecy::ExposeSecret;
use tiktools_collaboration::Credentials;

/// Explicitly secret-bearing; never Serialize/Debug the credential bundle.
#[derive(Clone)]
pub struct NativeCredentials {
    pub(super) origin: String,
    pub(super) host_id: String,
    pub(super) principal_id: String,
    pub(super) session_id: String,
    pub(super) access: SecretString,
    pub(super) refresh: SecretString,
    pub(super) access_expires_at: i64,
    pub(super) expires_at: i64,
}
impl std::fmt::Debug for NativeCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeCredentials([redacted])")
    }
}
impl NativeCredentials {
    pub(super) fn issued(
        origin: &str,
        host_id: &str,
        wire: Credentials,
    ) -> Result<Self, ClientError> {
        let refresh = wire.refresh_token.ok_or_else(protocol)?;
        let valid_token = |v: &str| {
            v.len() == 64
                && v.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !valid_id(host_id)
            || !valid_id(&wire.principal_id)
            || !valid_id(&wire.session_id)
            || !valid_token(&wire.access_token)
            || !valid_token(&refresh)
            || wire.access_expires_at <= now()
            || wire.expires_at <= now()
        {
            return Err(protocol());
        }
        Ok(Self {
            origin: origin.into(),
            host_id: host_id.into(),
            principal_id: wire.principal_id,
            session_id: wire.session_id,
            access: SecretString::from(wire.access_token),
            refresh: SecretString::from(refresh),
            access_expires_at: wire.access_expires_at,
            expires_at: wire.expires_at,
        })
    }
    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn access_expires_at(&self) -> i64 {
        self.access_expires_at
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
    pub(super) fn redact(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => {
                for secret in [&self.access, &self.refresh] {
                    *s = s.replace(secret.expose_secret(), "[redacted]");
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    self.redact(v);
                }
            }
            serde_json::Value::Object(o) => {
                let entries = std::mem::take(o);
                for (mut key, mut v) in entries {
                    for secret in [&self.access, &self.refresh] {
                        key = key.replace(secret.expose_secret(), "[redacted]");
                    }
                    self.redact(&mut v);
                    o.insert(key, v);
                }
            }
            _ => {}
        }
    }
}
