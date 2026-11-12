use super::*;
use crate::client::metadata::verified_room_creator;
use ttl_sign_core::room::{RoomInfo, RoomLookup, RoomOwner};

#[test]
fn selected_room_and_owner_must_both_match_discovery() {
    let room = "7300000000000000001";
    let lookup = RoomLookup {
        unique_id: "alice".into(),
        room_id: room.into(),
        nickname: String::new(),
        status: 2,
        title: String::new(),
    };
    let mut info = RoomInfo {
        room_id: room.into(),
        owner: RoomOwner {
            unique_id: "alice".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(verified_room_creator("alice", room, None, None), None);
    assert_eq!(
        verified_room_creator("alice", room, None, Some(&lookup)).as_deref(),
        Some("alice")
    );
    assert_eq!(
        verified_room_creator("alice", room, Some(&info), None).as_deref(),
        Some("alice")
    );
    assert_eq!(
        verified_room_creator("alice", room, Some(&info), Some(&lookup)).as_deref(),
        Some("alice")
    );
    info.room_id = "7300000000000000002".into();
    assert_eq!(
        verified_room_creator("alice", room, Some(&info), Some(&lookup)),
        None
    );
    info.room_id = room.into();
    info.owner.unique_id = "bob".into();
    assert_eq!(
        verified_room_creator("alice", room, Some(&info), None),
        None
    );
    assert_eq!(
        verified_room_creator("alice", room, Some(&info), Some(&lookup)),
        None
    );
    info.owner.unique_id.clear();
    assert_eq!(
        verified_room_creator("alice", room, Some(&info), Some(&lookup)),
        None
    );
    let mut wrong = lookup.clone();
    wrong.room_id = "7300000000000000002".into();
    assert_eq!(
        verified_room_creator("alice", room, None, Some(&wrong)),
        None
    );
    wrong = lookup.clone();
    wrong.unique_id = "bob".into();
    assert_eq!(
        verified_room_creator("alice", room, None, Some(&wrong)),
        None
    );
    wrong = lookup;
    wrong.status = 4;
    assert_eq!(
        verified_room_creator("alice", room, None, Some(&wrong)),
        None
    );
}

#[test]
fn connection_json_cannot_create_native_identity_evidence() {
    let info: ConnectionInfo = serde_json::from_value(serde_json::json!({
        "uniqueId": "alice", "verifiedCreator": "alice", "roomId": "room",
        "title": "", "nickname": "", "avatarUrl": null,
        "viewers": 0, "totalUsers": 0, "gifts": []
    }))
    .unwrap();
    assert_eq!(info.unique_id, "alice");
    assert!(info.verified_creator.is_none());
}
