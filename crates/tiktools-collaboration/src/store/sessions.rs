//! Credential issuance, refresh rotation, replay rejection and session revocation.
use super::*;

impl CollaborationService {
    pub fn refresh(&self, token: &str) -> Result<Credentials, Error> {
        if !valid_secret(token) {
            return Err(Error::Unauthorized);
        }
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "refresh-global", "host", 600)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let verifier = hash_secret(token);
        let used: Option<String> = tx
            .query_row(
                "SELECT session_id FROM used_refresh WHERE verifier=?1",
                [&verifier],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = used {
            tx.execute("UPDATE sessions SET revoked=1 WHERE id=?1", [&id])?;
            audit(
                &tx,
                "session-holder",
                "session.refresh-reuse",
                &id,
                "revoked",
            )?;
            tx.commit()?;
            self.invalidate_session(&id)?;
            return Err(Error::Unauthorized);
        }
        let row:Option<(String,String,i64)>=tx.query_row("SELECT s.id,s.principal_id,s.absolute_expires FROM sessions s JOIN principals p ON p.id=s.principal_id WHERE s.refresh_hash=?1 AND s.revoked=0 AND p.revoked=0 AND s.kind!='browser' AND s.absolute_expires>?2 AND s.last_activity>?3",params![verifier,now(),now().saturating_sub(IDLE_TTL)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let Some((session_id, principal_id, expires_at)) = row else {
            return Err(Error::Unauthorized);
        };
        let access = secret()?;
        let refresh = secret()?;
        let access_expires_at = (now() + ACCESS_TTL).min(expires_at);
        tx.execute(
            "INSERT INTO used_refresh(verifier,session_id) VALUES (?1,?2)",
            params![verifier, session_id],
        )?;
        tx.execute("UPDATE sessions SET access_hash=?2,access_expires=?3,refresh_hash=?4,last_activity=?5 WHERE id=?1",params![session_id,hash_secret(&access),access_expires_at,hash_secret(&refresh),now()])?;
        audit(
            &tx,
            &principal_id,
            "session.refresh",
            &session_id,
            "succeeded",
        )?;
        tx.commit()?;
        self.invalidate_session(&session_id)?;
        Ok(Credentials {
            principal_id,
            session_id,
            access_token: access,
            access_expires_at,
            refresh_token: Some(refresh),
            expires_at,
        })
    }

    pub fn sessions(&self, input: ListQuery) -> Result<ListPage<Session>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT s.id,s.principal_id,s.kind,s.created_at,s.last_activity,s.absolute_expires,s.revoked,COALESCE(p.label,'') FROM sessions s LEFT JOIN principals p ON p.id=s.principal_id WHERE s.id>?1 ORDER BY s.id LIMIT ?2")?;
        let result = page(
            stmt.query_map(params![after, (limit + 1) as i64], |r| {
                let id: String = r.get(0)?;
                Ok((
                    id.clone(),
                    Session {
                        session_id: id,
                        principal_id: r.get(1)?,
                        client_label: r.get(7)?,
                        client_kind: parse_kind(&r.get::<_, String>(2)?)?,
                        created_at: r.get(3)?,
                        last_activity_at: r.get(4)?,
                        expires_at: r.get(5)?,
                        revoked: r.get(6)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn revoke_session(&self, id: &str, actor: &str) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE sessions SET revoked=1 WHERE id=?1", [id])? == 0 {
            return Err(Error::NotFound);
        }
        audit(&tx, actor, "session.revoke", id, "succeeded")?;
        tx.commit()?;
        self.invalidate_session(id)?;
        Ok(())
    }
}

pub(super) fn issue(
    tx: &Transaction<'_>,
    principal_id: &str,
    client_kind: ClientKind,
) -> Result<Credentials, Error> {
    let session_id = Uuid::new_v4().to_string();
    let access_token = secret()?;
    let refresh_token = if client_kind == ClientKind::Browser {
        None
    } else {
        Some(secret()?)
    };
    let expires_at = now() + SESSION_TTL;
    let access_expires_at = if client_kind == ClientKind::Browser {
        expires_at
    } else {
        now() + ACCESS_TTL
    };
    tx.execute("INSERT INTO sessions(id,principal_id,kind,access_hash,access_expires,refresh_hash,created_at,last_activity,absolute_expires) VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8)",params![session_id,principal_id,kind(client_kind),hash_secret(&access_token),access_expires_at,refresh_token.as_deref().map(hash_secret),now(),expires_at])?;
    Ok(Credentials {
        principal_id: principal_id.into(),
        session_id,
        access_token,
        access_expires_at,
        refresh_token,
        expires_at,
    })
}
