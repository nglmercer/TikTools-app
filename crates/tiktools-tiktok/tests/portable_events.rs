//! Native regression fixtures; no TikTok session or network needed.
use ttl_live_events::{decode_event, LiveEvent};

#[test]
fn principal_live_events_preserve_unicode_and_original_metadata() {
    match decode_event(
        "WebcastChatMessage",
        include_bytes!("fixtures/chat-unicode-emote.pb"),
    ) {
        LiveEvent::Chat(event) => {
            assert_eq!(event.comment, "こんにちは [laughcry]");
            assert_eq!(event.emotes[0].id, "7098");
        }
        other => panic!("chat decoded as {other:?}"),
    }
    match decode_event("WebcastGiftMessage", include_bytes!("fixtures/gift.pb")) {
        LiveEvent::Gift(gift) => {
            assert_eq!(gift.gift_id, 5655);
            assert_eq!(gift.gift_name, "Rose");
            assert_eq!(gift.repeat_count, 7);
            assert_eq!(gift.combo_count, 3);
            assert!(gift.repeat_end);
        }
        other => panic!("gift decoded as {other:?}"),
    }
    assert!(
        matches!(decode_event("WebcastLikeMessage", include_bytes!("fixtures/like.pb")), LiveEvent::Like(event) if event.count == 12 && event.total == 4821)
    );
    for (bytes, kind) in [
        (
            include_bytes!("fixtures/social.pb").as_slice(),
            tiktools_tiktok::events::SocialKind::Follow,
        ),
        (
            include_bytes!("fixtures/share.pb").as_slice(),
            tiktools_tiktok::events::SocialKind::Share,
        ),
    ] {
        let LiveEvent::Social(event) = decode_event("WebcastSocialMessage", bytes) else {
            panic!("social fixture")
        };
        assert_eq!(
            tiktools_tiktok::events::classify_social_action(event.action),
            kind
        );
    }
    assert!(matches!(
        decode_event(
            "WebcastRoomUserSeqMessage",
            include_bytes!("fixtures/room-user.pb")
        ),
        LiveEvent::RoomUser(_)
    ));
    assert!(matches!(
        decode_event("WebcastMemberMessage", include_bytes!("fixtures/member.pb")),
        LiveEvent::Member(_)
    ));
}

#[test]
fn unnormalized_subscription_and_room_control_payloads_are_preserved() {
    // The upstream contract keeps unknown methods, rather than inventing a
    // subscription schema. Future normalizers must retain this raw payload.
    for method in ["WebcastSubNotifyMessage", "WebcastControlMessage"] {
        let payload = [8, 1];
        assert_eq!(
            decode_event(method, &payload),
            LiveEvent::Unknown {
                method: method.into(),
                payload: payload.to_vec(),
            }
        );
    }
}
