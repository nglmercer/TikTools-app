use serde_json::{json, Value};
use tiktools_live::{LiveEnvelope, LivePayload};

pub(super) fn ui_event(event: &LiveEnvelope, correlation: &str) -> Option<Value> {
    let user = event.user.as_ref()?;
    let (kind, text, key, params) = match &event.payload {
        LivePayload::Chat { comment, .. } => (
            "chat",
            comment.clone(),
            "chatMessage",
            json!({"comment":comment}),
        ),
        LivePayload::Gift {
            name,
            count,
            unit,
            amount,
            ..
        } => (
            "gift",
            format!("sent {count}× {name} ({amount} {unit})"),
            "platformGiftSent",
            json!({"count":count,"giftName":name,"amount":amount,"unit":unit}),
        ),
        LivePayload::Subscription { .. } => (
            "member",
            "subscribed".into(),
            "platformSubscribed",
            json!({}),
        ),
        LivePayload::Follow => (
            "social",
            "followed the creator".into(),
            "followedCreator",
            json!({}),
        ),
        LivePayload::Share { .. } => ("social", "shared the live".into(), "sharedLive", json!({})),
        LivePayload::Join => ("member", "joined the live".into(), "joinedLive", json!({})),
        LivePayload::Like { count, .. } => (
            "like",
            format!("sent {count} likes"),
            "likeSent",
            json!({"count":count}),
        ),
        _ => return None,
    };
    let mut ui = json!({"kind":kind,"author":user.key(),"nickname":user.display_name,"avatarUrl":user.avatar_url,"text":text,"platform":event.platform,"sourceType":event.payload.event_type(),"msgId":event.source_event_id,"roomId":event.channel.room_id,"creatorId":tiktools_live::identity_key(event.platform,&event.channel.id),"correlationId":correlation,"isHistory":event.is_history,"i18nKey":key,"i18nParams":params});
    if let LivePayload::Gift {
        id,
        name,
        count,
        unit,
        amount,
    } = &event.payload
    {
        ui["giftDetails"] = json!({"giftId":id,"name":name,"count":count,"diamonds":0,"unit":unit,"amount":amount,"streakable":false});
    }
    if let LivePayload::Like { count, .. } = &event.payload {
        ui["likeCount"] = json!(count);
        ui["likeViewerKey"] = json!(user.key());
    }
    Some(ui)
}
