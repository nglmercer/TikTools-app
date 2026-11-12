//! Bounded list pagination and SQLite row decoding.
use super::*;

/// Read lazily so large grants reduce a page rather than allocate every row.
/// The envelope stays below the default 1 MiB transport budget.
pub(super) fn page<T: serde::Serialize>(
    rows: impl Iterator<Item = rusqlite::Result<(String, T)>>,
    limit: usize,
) -> Result<ListPage<T>, Error> {
    const MAX_PAGE_BYTES: usize = 512 * 1024;
    let mut items = Vec::new();
    let mut bytes = 0;
    let mut last = None;
    let mut next_after = None;
    for row in rows {
        let (id, value) = row?;
        let size = serde_json::to_vec(&value)?.len();
        if items.len() == limit || bytes + size > MAX_PAGE_BYTES {
            if items.is_empty() {
                return Err(Error::Invalid(
                    "collaboration list item exceeds page budget",
                ));
            }
            next_after = last;
            break;
        }
        bytes += size;
        last = Some(id);
        items.push(value);
    }
    Ok(ListPage { items, next_after })
}

pub(super) fn kind(kind: ClientKind) -> &'static str {
    match kind {
        ClientKind::Desktop => "desktop",
        ClientKind::Browser => "browser",
        ClientKind::Agent => "agent",
    }
}

pub(super) fn parse_kind(value: &str) -> rusqlite::Result<ClientKind> {
    match value {
        "desktop" => Ok(ClientKind::Desktop),
        "browser" => Ok(ClientKind::Browser),
        "agent" => Ok(ClientKind::Agent),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

pub(super) fn grant(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Grant> {
    let json: String = row.get(index)?;
    serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
