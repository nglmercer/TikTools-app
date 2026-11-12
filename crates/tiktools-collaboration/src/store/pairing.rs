//! Pairing request admission, host approval and credential collection.
use super::*;

impl CollaborationService {
    /// source is supplied by the trusted network boundary, never by the JSON
    /// payload. Limits persist even when the following exchange is rejected.
    pub fn redeem(&self, input: Redeem, source: &str) -> Result<PairingRequest, Error> {
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "redeem-global", "host", 60)?;
        rate(&db, "redeem-source", source, 5)?;
        if input.client_label.trim().is_empty()
            || input.client_label.len() > 80
            || input.client_label.chars().any(char::is_control)
            || !valid_secret(&input.secret)
            || !valid_secret(&input.exchange_secret_hash)
        {
            return Err(Error::PairingExpired);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count:i64 = tx.query_row("SELECT count(*) FROM pairing_requests WHERE state IN ('pending','approved') AND expires_at>?1",[now()],|row|row.get(0))?;
        if count >= 32 {
            return Err(Error::RateLimited);
        }
        let invitation:Option<String> = tx.query_row("SELECT grant_json FROM invitations WHERE id=?1 AND verifier=?2 AND state='created' AND expires_at>?3",params![input.invite_id,hash_secret(&input.secret),now()],|row|row.get(0)).optional()?;
        let Some(grant_json) = invitation else {
            return Err(Error::PairingExpired);
        };
        tx.execute(
            "UPDATE invitations SET state='redeemed' WHERE id=?1",
            [&input.invite_id],
        )?;
        let request = PairingRequest {
            request_id: Uuid::new_v4().to_string(),
            client_label: input.client_label.trim().into(),
            client_kind: input.client_kind,
            grant: serde_json::from_str(&grant_json)?,
            state: "pending".into(),
            expires_at: now() + INVITE_TTL,
        };
        tx.execute("INSERT INTO pairing_requests(id,invite_id,exchange_hash,label,kind,grant_json,state,expires_at) VALUES (?1,?2,?3,?4,?5,?6,'pending',?7)",params![request.request_id,input.invite_id,input.exchange_secret_hash,request.client_label,kind(request.client_kind),grant_json,request.expires_at])?;
        audit(
            &tx,
            "pending-controller",
            "pair.redeem",
            &request.request_id,
            "pending",
        )?;
        tx.commit()?;
        Ok(request)
    }

    pub fn requests(&self, input: ListQuery) -> Result<ListPage<PairingRequest>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT id,label,kind,grant_json,state,expires_at FROM pairing_requests WHERE state='pending' AND expires_at>?1 AND id>?2 ORDER BY id LIMIT ?3")?;
        let result = page(
            stmt.query_map(params![now(), after, (limit + 1) as i64], |row| {
                let id: String = row.get(0)?;
                Ok((
                    id.clone(),
                    PairingRequest {
                        request_id: id,
                        client_label: row.get(1)?,
                        client_kind: parse_kind(&row.get::<_, String>(2)?)?,
                        grant: grant(row, 3)?,
                        state: row.get(4)?,
                        expires_at: row.get(5)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn decide_request(&self, id: &str, approve: bool) -> Result<(), Error> {
        let mut db = self.db()?;
        enabled(&db)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,String,String)> = tx.query_row("SELECT label,kind,grant_json FROM pairing_requests WHERE id=?1 AND state='pending' AND expires_at>?2",params![id,now()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
        let Some((label, client_kind, grant_json)) = row else {
            return Err(Error::PairingExpired);
        };
        let principal_id = Uuid::new_v4().to_string();
        if approve {
            let count: i64 =
                tx.query_row("SELECT count(*) FROM principals WHERE revoked=0", [], |r| {
                    r.get(0)
                })?;
            if count >= 256 {
                return Err(Error::RateLimited);
            }
            tx.execute(
                "INSERT INTO principals(id,label,kind,grant_json) VALUES (?1,?2,?3,?4)",
                params![principal_id, label, client_kind, grant_json],
            )?;
        }
        tx.execute(
            "UPDATE pairing_requests SET state=?2,principal_id=?3 WHERE id=?1",
            params![
                id,
                if approve { "approved" } else { "rejected" },
                approve.then_some(principal_id)
            ],
        )?;
        audit(
            &tx,
            "local-owner",
            if approve {
                "pair.approve"
            } else {
                "pair.reject"
            },
            id,
            "succeeded",
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn collect(&self, input: Collect, source: &str) -> Result<Credentials, Error> {
        self.collect_inner(input, source, None)
    }

    /// The network boundary chooses the authentication mode, not clientKind.
    pub fn collect_for(
        &self,
        input: Collect,
        source: &str,
        browser: bool,
    ) -> Result<Credentials, Error> {
        self.collect_inner(input, source, Some(browser))
    }

    fn collect_inner(
        &self,
        input: Collect,
        source: &str,
        browser: Option<bool>,
    ) -> Result<Credentials, Error> {
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "collect-global", "host", 600)?;
        rate(&db, "collect-source", source, 60)?;
        if !valid_secret(&input.exchange_secret) {
            return Err(Error::PairingExpired);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,Option<String>,String,i64)> = tx.query_row("SELECT state,principal_id,kind,expires_at FROM pairing_requests WHERE id=?1 AND exchange_hash=?2",params![input.request_id,hash_secret(&input.exchange_secret)],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional()?;
        let Some((state, principal, client_kind, expires)) = row else {
            return Err(Error::PairingExpired);
        };
        let client_kind = parse_kind(&client_kind)?;
        if browser.is_some_and(|browser| (client_kind == ClientKind::Browser) != browser) {
            return Err(Error::PairingExpired);
        }
        if expires <= now() {
            return Err(Error::PairingExpired);
        }
        match state.as_str() {
            "pending" => return Err(Error::PairingPending),
            "rejected" => return Err(Error::PairingRejected),
            "approved" => {}
            _ => return Err(Error::PairingExpired),
        }
        let principal = principal.ok_or(Error::Unauthorized)?;
        let active: bool = tx.query_row(
            "SELECT revoked=0 FROM principals WHERE id=?1",
            [&principal],
            |r| r.get(0),
        )?;
        if !active {
            return Err(Error::Unauthorized);
        }
        let credentials = issue(&tx, &principal, client_kind)?;
        tx.execute(
            "UPDATE pairing_requests SET state='collected',exchange_hash='' WHERE id=?1",
            [&input.request_id],
        )?;
        audit(
            &tx,
            &principal,
            "session.create",
            &credentials.session_id,
            "succeeded",
        )?;
        tx.commit()?;
        Ok(credentials)
    }
}
