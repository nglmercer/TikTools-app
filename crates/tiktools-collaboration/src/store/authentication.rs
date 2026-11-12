//! Bearer authentication and shared, immediately revocable authorization views.
use super::*;

impl CollaborationService {
    pub fn authenticate(
        &self,
        token: &str,
        browser: bool,
        touch: bool,
    ) -> Result<Authenticated, Error> {
        let db = self.db()?;
        authenticate_db(&db, token, browser, touch)
    }

    /// Authenticate on the blocking pool, then retain a shared revocable
    /// view for authorization/event delivery without per-event disk reads.
    pub fn authenticate_live(
        &self,
        token: &str,
        browser: bool,
        touch: bool,
    ) -> Result<Authorization, Error> {
        let db = self.db()?;
        let identity = authenticate_db(&db, token, browser, touch)?;
        let verifier = hash_secret(token);
        let mut authorizations = self
            .authorizations
            .lock()
            .map_err(|_| Error::Unauthorized)?;
        authorizations.retain(|_, state| state.strong_count() > 0);
        if let Some(state) = authorizations.get(&verifier).and_then(Weak::upgrade) {
            if state.revoked.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(Error::Unauthorized);
            }
            *state.identity.write().map_err(|_| Error::Unauthorized)? = identity;
            return Ok(Authorization { state });
        }
        let authorization = Authorization::new(identity);
        authorizations.insert(verifier, Arc::downgrade(&authorization.state));
        Ok(authorization)
    }

    pub(super) fn update_authorizations(
        &self,
        update: impl Fn(&crate::authority::AuthorizationState),
    ) -> Result<(), Error> {
        let mut views = match self.authorizations.lock() {
            Ok(views) => views,
            Err(poisoned) => {
                for view in poisoned.into_inner().values().filter_map(Weak::upgrade) {
                    view.revoke();
                }
                return Err(Error::Unauthorized);
            }
        };
        views.retain(|_, view| {
            if let Some(view) = view.upgrade() {
                update(&view);
                true
            } else {
                false
            }
        });
        Ok(())
    }

    pub(super) fn invalidate_session(&self, id: &str) -> Result<(), Error> {
        self.update_authorizations(|state| match state.identity.read() {
            Ok(identity) if identity.session_id == id => state.revoke(),
            Err(_) => state.revoke(),
            _ => {}
        })
    }
}

fn authenticate_db(
    db: &Connection,
    token: &str,
    browser: bool,
    touch: bool,
) -> Result<Authenticated, Error> {
    if !valid_secret(token) {
        return Err(Error::Unauthorized);
    }
    enabled(db)?;
    let result:Option<(Authenticated,i64,i64)>=db.query_row("SELECT p.id,s.id,p.grant_json,p.revision,s.kind,s.access_expires,s.last_activity,s.created_at FROM sessions s JOIN principals p ON p.id=s.principal_id WHERE s.access_hash=?1 AND s.revoked=0 AND p.revoked=0 AND s.access_expires>?2 AND s.absolute_expires>?2 AND s.last_activity>?3",params![hash_secret(token),now(),now().saturating_sub(IDLE_TTL)],|r|Ok((Authenticated {principal_id:r.get(0)?,session_id:r.get(1)?,grant:grant(r,2)?,grant_revision:r.get::<_,i64>(3)?.to_string(),client_kind:parse_kind(&r.get::<_,String>(4)?)?,access_expires_at:r.get(5)?},r.get(6)?,r.get(7)?))).optional()?;
    let (mut identity, last_activity, created_at) = result.ok_or(Error::Unauthorized)?;
    if (identity.client_kind == ClientKind::Browser) != browser {
        return Err(Error::Unauthorized);
    }
    let last_activity = if touch {
        let at = now();
        if identity.client_kind == ClientKind::Browser {
            // No refresh token exists for a browser. Prove the session is live
            // by using it, and slide both expiries together, bounded by a hard
            // ceiling measured from pairing.
            let extended = (at + IDLE_TTL).min(created_at + BROWSER_SESSION_MAX);
            db.execute(
                "UPDATE sessions SET last_activity=?2, absolute_expires=MAX(absolute_expires,?3), access_expires=MAX(access_expires,?3) WHERE id=?1",
                params![identity.session_id, at, extended],
            )?;
            identity.access_expires_at = identity.access_expires_at.max(extended);
        } else {
            db.execute(
                "UPDATE sessions SET last_activity=?2 WHERE id=?1",
                params![identity.session_id, at],
            )?;
        }
        at
    } else {
        last_activity
    };
    identity.access_expires_at = identity.access_expires_at.min(last_activity + IDLE_TTL);
    Ok(identity)
}
