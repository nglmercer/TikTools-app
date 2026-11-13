use super::*;
use crate::relay::{io::Record, Announcement};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForwardedAnnouncement {
    pub r#type: String,
    pub announcement: Announcement,
    pub from_endpoint_id: String,
    pub expires_at: i64,
}
pub(super) fn parse(value: Value, ready: &Ready) -> Result<Event, Error> {
    match value["type"].as_str() {
        Some("relay.open") => {
            let frame: ForwardedAnnouncement =
                serde_json::from_value(value).map_err(|_| Error::Negotiation)?;
            let context = &frame.announcement.context;
            if context.host_id != ready.host_id
                || ready.session_id.is_some_and(|id| id != context.session_id)
                || ready.session_id.is_some() != (context.from_device_id == context.host_device_id)
                || context.grant_revision <= 0
                || frame.announcement.signature.len() != 86
                || frame.announcement.certificate.len() > 4096 * 4 / 3 + 4
                || frame.from_endpoint_id.len() != 64
                || !frame
                    .from_endpoint_id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(Error::Negotiation);
            }
            Ok(Event::RelayOpen(Box::new(frame)))
        }
        Some("relay.data" | "relay.close") => {
            let record: Record = serde_json::from_value(value).map_err(|_| Error::Negotiation)?;
            let (_, session, _, sequence) = record.position();
            if ready.session_id.is_some_and(|id| id != session)
                || sequence < 0
                || sequence == i64::MAX
            {
                return Err(Error::Negotiation);
            }
            Ok(Event::RelayRecord(record))
        }
        Some("relay.accepted") => {
            let tunnel_id = value["tunnelId"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .ok_or(Error::Negotiation)?;
            let sequence = match value.get("sequence") {
                Some(Value::Null) => None,
                Some(n) => Some(
                    n.as_i64()
                        .filter(|n| *n >= 0 && *n < i64::MAX)
                        .ok_or(Error::Negotiation)?,
                ),
                None => return Err(Error::Negotiation),
            };
            Ok(Event::RelayAccepted {
                tunnel_id,
                sequence,
            })
        }
        _ => Err(Error::Negotiation),
    }
}
