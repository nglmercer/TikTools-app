use super::*;
use crate::peer_clock::PeerLease;

/// No binding means a legacy session. A peer binding without clock evidence is
/// deliberately denied until the native host revalidates its Cloud grant.
pub(crate) fn for_session(db: &Connection, session: &str) -> Result<Option<PeerLease>, Error> {
    let lease = db.query_row(
        "SELECT l.boot_id,l.issued_ms,l.until_ms FROM cloud_peer_bindings b LEFT JOIN cloud_peer_leases l ON l.session_id=b.session_id WHERE b.session_id=?1 AND b.valid_until>0",
        [session],
        |r| Ok((r.get::<_,Option<String>>(0)?, r.get::<_,Option<i64>>(1)?, r.get::<_,Option<i64>>(2)?)),
    ).optional()?;
    let Some((boot, issued_ms, until_ms)) = lease else {
        let peer: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM cloud_peer_bindings WHERE session_id=?1)",
            [session],
            |r| r.get(0),
        )?;
        return if peer {
            Err(Error::Unauthorized)
        } else {
            Ok(None)
        };
    };
    let lease = PeerLease {
        boot: boot.ok_or(Error::Unauthorized)?,
        issued_ms: issued_ms.ok_or(Error::Unauthorized)?,
        until_ms: until_ms.ok_or(Error::Unauthorized)?,
    };
    lease.remaining()?;
    Ok(Some(lease))
}

pub(super) fn save(db: &Connection, session: &str, lease: &PeerLease) -> Result<(), Error> {
    db.execute("INSERT INTO cloud_peer_leases(session_id,boot_id,issued_ms,until_ms) VALUES(?1,?2,?3,?4) ON CONFLICT(session_id) DO UPDATE SET boot_id=excluded.boot_id,issued_ms=excluded.issued_ms,until_ms=excluded.until_ms", params![session,lease.boot,lease.issued_ms,lease.until_ms])?;
    Ok(())
}
