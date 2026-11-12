//! Creation, listing and revocation of one-use pairing invitations.
use super::*;

impl CollaborationService {
    pub fn create_invitation(&self, input: InviteCreate) -> Result<InviteSecret, Error> {
        input.grant.validate()?;
        let mut db = self.db()?;
        enabled(&db)?;
        rate(&db, "invite", "owner", 10)?;
        let active: i64 = db.query_row(
            "SELECT count(*) FROM invitations WHERE state='created' AND expires_at>?1",
            [now()],
            |row| row.get(0),
        )?;
        if active >= 32 {
            return Err(Error::RateLimited);
        }
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let invite_id = Uuid::new_v4().to_string();
        let token = secret()?;
        let expires_at = now() + INVITE_TTL;
        tx.execute("INSERT INTO invitations(id,verifier,grant_json,created_at,expires_at,state) VALUES (?1,?2,?3,?4,?5,'created')", params![invite_id, hash_secret(&token), serde_json::to_string(&input.grant)?, now(), expires_at])?;
        audit(&tx, "local-owner", "invite.create", &invite_id, "succeeded")?;
        tx.commit()?;
        Ok(InviteSecret {
            invite_id,
            secret: token,
            expires_at,
        })
    }

    pub fn invitations(&self, input: ListQuery) -> Result<ListPage<Invitation>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt = db.prepare("SELECT id,grant_json,expires_at,CASE WHEN state='created' AND expires_at<=?1 THEN 'expired' ELSE state END FROM invitations WHERE id>?2 ORDER BY id LIMIT ?3")?;
        let result = page(
            stmt.query_map(params![now(), after, (limit + 1) as i64], |row| {
                let id: String = row.get(0)?;
                Ok((
                    id.clone(),
                    Invitation {
                        invite_id: id,
                        grant: grant(row, 1)?,
                        expires_at: row.get(2)?,
                        state: row.get(3)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn revoke_invitation(&self, id: &str) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE invitations SET state='revoked' WHERE id=?1", [id])? == 0 {
            return Err(Error::NotFound);
        }
        tx.execute("UPDATE pairing_requests SET state='revoked' WHERE invite_id=?1 AND state IN ('pending','approved')", [id])?;
        audit(&tx, "local-owner", "invite.revoke", id, "succeeded")?;
        tx.commit()?;
        Ok(())
    }
}
