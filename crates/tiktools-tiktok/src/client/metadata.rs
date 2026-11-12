//! Native room/creator verification and gift metadata adaptation.
use crate::GiftInfo;

pub(in super::super) fn gift_info(gift: ttl_sign_core::Gift) -> GiftInfo {
    let streakable = gift.is_streakable();
    GiftInfo {
        id: gift.id.to_string(),
        name: gift.name,
        diamond_count: gift.diamond_count,
        icon_url: (!gift.icon_url.is_empty()).then_some(gift.icon_url),
        streakable,
    }
}

pub(in super::super) fn clean_unique_id(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('@');
    (!value.is_empty()).then_some(value.to_owned())
}

/// Only discovery results bound to the selected room can authorize a
/// creator-scoped controller or delegated execution. The requested handle and
/// a caller-provided room id are not evidence of each other's identity.
pub(in super::super) fn verified_room_creator(
    requested: &str,
    selected_room: &str,
    info: Option<&ttl_sign_core::room::RoomInfo>,
    lookup: Option<&ttl_sign_core::room::RoomLookup>,
) -> Option<String> {
    let requested = clean_unique_id(requested)?;
    if requested.len() > 256 || requested.chars().any(char::is_control) {
        return None;
    }
    let lookup_creator = match lookup {
        Some(found) => {
            if !found.is_live() || found.room_id != selected_room {
                return None;
            }
            let owner = clean_unique_id(&found.unique_id)?;
            if owner != requested {
                return None;
            }
            Some(owner)
        }
        None => None,
    };
    match info {
        Some(found) => {
            if found.room_id != selected_room {
                return None;
            }
            let owner = clean_unique_id(&found.owner.unique_id)?;
            if owner != requested {
                return None;
            }
            if lookup_creator
                .as_ref()
                .is_some_and(|lookup| lookup != &owner)
            {
                return None;
            }
            Some(owner)
        }
        None => lookup_creator,
    }
}
