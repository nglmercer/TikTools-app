//! Principal lookup, grant revisions and principal-wide revocation.
use super::*;

impl CollaborationService {
    pub fn principals(&self, input: ListQuery) -> Result<ListPage<Principal>, Error> {
        let (after, limit) = input.bounds()?;
        let db = self.db()?;
        let mut stmt=db.prepare("SELECT id,label,kind,grant_json,revision,revoked FROM principals WHERE id>?1 ORDER BY id LIMIT ?2")?;
        let result = page(
            stmt.query_map(params![after, (limit + 1) as i64], |r| {
                let id: String = r.get(0)?;
                Ok((
                    id.clone(),
                    Principal {
                        principal_id: id,
                        client_label: r.get(1)?,
                        client_kind: parse_kind(&r.get::<_, String>(2)?)?,
                        grant: grant(r, 3)?,
                        grant_revision: r.get::<_, i64>(4)?.to_string(),
                        revoked: r.get(5)?,
                    },
                ))
            })?,
            limit,
        );
        result
    }

    pub fn principal(&self, id: &str) -> Result<Principal, Error> {
        if uuid::Uuid::parse_str(id).map_or(true, |parsed| parsed.to_string() != id) {
            return Err(Error::Invalid("invalid principal ID"));
        }
        let db = self.db()?;
        let length: Option<i64> = db
            .query_row(
                "SELECT length(CAST(grant_json AS BLOB)) FROM principals WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        let length = length.ok_or(Error::NotFound)?;
        // Reserve space for the label, identity and JSON-RPC envelope before
        // parsing a stored grant. The service's DB lock spans both reads.
        if length > 512 * 1024 - 2048 {
            return Err(Error::TooLarge);
        }
        db.query_row(
            "SELECT id,label,kind,grant_json,revision,revoked FROM principals WHERE id=?1",
            [id],
            |r| {
                Ok(Principal {
                    principal_id: r.get(0)?,
                    client_label: r.get(1)?,
                    client_kind: parse_kind(&r.get::<_, String>(2)?)?,
                    grant: grant(r, 3)?,
                    grant_revision: r.get::<_, i64>(4)?.to_string(),
                    revoked: r.get(5)?,
                })
            },
        )
        .map_err(Error::from)
    }

    pub fn revoke_principal(&self, id: &str) -> Result<(), Error> {
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE principals SET revoked=1,revision=revision+1 WHERE id=?1",
            [id],
        )? == 0
        {
            return Err(Error::NotFound);
        }
        tx.execute("UPDATE sessions SET revoked=1 WHERE principal_id=?1", [id])?;
        tx.execute("UPDATE pairing_requests SET state='revoked' WHERE principal_id=?1 AND state='approved'",[id])?;
        audit(&tx, "local-owner", "principal.revoke", id, "succeeded")?;
        tx.commit()?;
        self.update_authorizations(|state| match state.identity.read() {
            Ok(identity) if identity.principal_id == id => state.revoke(),
            Err(_) => state.revoke(),
            _ => {}
        })?;
        Ok(())
    }

    pub fn set_grant(&self, input: GrantSet) -> Result<(), Error> {
        input.grant.validate()?;
        let revision: i64 = input
            .expected_revision
            .parse()
            .map_err(|_| Error::Invalid("invalid grant revision"))?;
        if revision <= 0 || revision == i64::MAX {
            return Err(Error::Invalid("invalid grant revision"));
        }
        let mut db = self.db()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE principals SET grant_json=?2,revision=revision+1 WHERE id=?1 AND revision=?3 AND revoked=0",params![input.principal_id,serde_json::to_string(&input.grant)?,revision])?==0{return Err(Error::RevisionConflict);}
        audit(
            &tx,
            "local-owner",
            "grant.set",
            &input.principal_id,
            "succeeded",
        )?;
        tx.commit()?;
        self.update_authorizations(|state| {
            state.update_grant(
                &input.principal_id,
                &input.grant,
                &(revision + 1).to_string(),
            )
        })?;
        Ok(())
    }
}
