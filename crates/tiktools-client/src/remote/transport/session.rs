use super::*;

/// Shared RPC identity; P2P never invents bearer/refresh credentials.
#[derive(Clone)]
pub(super) struct SessionIdentity {
    pub host_id: String,
    pub principal_id: String,
    pub session_id: String,
    // Bearer deadlines apply only to HTTPS. The P2P host continuously enforces
    // its renewable boot-clock lease; comparing remote wall time here would
    // break valid sessions after clock correction or the initial eight hours.
    pub access_expires_at: Option<i64>,
    pub redactor: Option<NativeCredentials>,
}

impl From<&NativeCredentials> for SessionIdentity {
    fn from(credentials: &NativeCredentials) -> Self {
        Self {
            host_id: credentials.host_id.clone(),
            principal_id: credentials.principal_id.clone(),
            session_id: credentials.session_id.clone(),
            access_expires_at: Some(credentials.access_expires_at),
            redactor: Some(credentials.clone()),
        }
    }
}

impl SessionIdentity {
    pub fn redact(&self, value: &mut Value) {
        if let Some(credentials) = &self.redactor {
            credentials.redact(value);
        }
    }
}
