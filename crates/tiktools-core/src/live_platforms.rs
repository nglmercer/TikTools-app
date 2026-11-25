//! Host-owned live platform ingestion.
//!
//! Provider plugins supply bounded, declarative batches. This module is the
//! trust boundary: it validates shape and capability, binds the emitting
//! plugin as platform owner, assigns sessions/timestamps/authority, then
//! projects accepted events into feed, automation, points and analytics.

use crate::*;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, VecDeque};
use tiktools_live::{
    identity_key, Capabilities, ChannelIdentity, LiveEnvelope, LiveEventInput, LiveIngestBatch,
    LivePayload, PlatformError, PlatformIngestMessage, PlatformSessionSnapshot, SessionState,
    ViewerIdentity,
};

// Keep this module independent from provider crates. `tiktools_live` is the
// versioned contract shared with external plugin implementations.
use tiktools_live::{INGEST_EVENT_TYPE, MAX_BATCH_MESSAGES, MAX_STREAM_CHARS, SCHEMA_VERSION};

const MAX_TEXT_CHARS: usize = 4096;
const MAX_ID_CHARS: usize = 256;
const MAX_NAME_CHARS: usize = 128;
const MAX_URL_CHARS: usize = 2048;
const MAX_SESSIONS_PER_PLUGIN: usize = 32;
const MAX_SESSIONS_TOTAL: usize = 128;
const DEDUP_CAPACITY: usize = 50_000;
const DEDUP_TTL: std::time::Duration = std::time::Duration::from_secs(600);

// The constant lives in plugin-api, not in the provider-neutral contract crate.
const LIVE_INGEST_CAPABILITY: &str = tiktools_plugin_api::capabilities::LIVE_INGEST;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DedupKey {
    platform: String,
    channel: String,
    session: String,
    event_type: &'static str,
    source: String,
}

#[derive(Debug, Clone)]
struct PlatformSession {
    snapshot: PlatformSessionSnapshot,
    ended: bool,
}

#[derive(Default)]
pub(crate) struct LivePlatformService {
    sessions: Mutex<BTreeMap<(String, String), PlatformSession>>,
    dedup: Mutex<(HashMap<DedupKey, Instant>, VecDeque<DedupKey>)>,
    owner: Mutex<Option<std::sync::Weak<AppCore>>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LiveIngestOutcome {
    pub(crate) accepted: usize,
    pub(crate) duplicates: usize,
    pub(crate) rejected: usize,
}

impl LivePlatformService {
    fn set_owner(&self, owner: &Arc<AppCore>) {
        *recover_mutex(&self.owner, "live platform owner") = Some(Arc::downgrade(owner));
    }

    fn owner(&self) -> Option<Arc<AppCore>> {
        recover_mutex(&self.owner, "live platform owner")
            .as_ref()
            .and_then(std::sync::Weak::upgrade)
    }

    fn end_plugin_sessions(&self, plugin_id: &str) -> Vec<PlatformSessionSnapshot> {
        let mut sessions = recover_mutex(&self.sessions, "live platform sessions");
        let keys = sessions
            .keys()
            .filter(|(plugin, _)| plugin == plugin_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut ended = Vec::new();
        for key in keys {
            if sessions.get(&key).is_some_and(|session| session.ended) {
                // SessionEnd marks the session before awaiting analytics. Leave
                // it in place so that in-flight path remains responsible for
                // dispatching exactly one disconnect and removing the session.
                continue;
            }
            if let Some(mut session) = sessions.remove(&key) {
                session.snapshot.state = SessionState::Disconnected;
                session.snapshot.updated_at = tiktools_live::now_millis();
                ended.push(session.snapshot);
            }
        }
        ended
    }

    fn duplicate(&self, event: &LiveEnvelope) -> bool {
        let Some(source) = event
            .source_event_id
            .as_deref()
            .filter(|source| !source.is_empty())
        else {
            return false;
        };
        let key = DedupKey {
            platform: event.platform.clone(),
            channel: event.channel.id.clone(),
            session: event.session_id.clone(),
            event_type: event.payload.event_type(),
            source: source.to_owned(),
        };
        let mut seen = recover_mutex(&self.dedup, "live platform dedup");
        while let Some(front) = seen.1.front().cloned() {
            if seen.0.len() < DEDUP_CAPACITY
                && seen
                    .0
                    .get(&front)
                    .is_some_and(|at| at.elapsed() < DEDUP_TTL)
            {
                break;
            }
            seen.1.pop_front();
            seen.0.remove(&front);
        }
        if seen.0.contains_key(&key) {
            return true;
        }
        seen.0.insert(key.clone(), Instant::now());
        seen.1.push_back(key);
        false
    }
}

pub(crate) fn is_live_ingest_event(event_type: &str) -> bool {
    event_type == INGEST_EVENT_TYPE
}

fn bounded(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn optional_bounded(value: &str, max: usize) -> bool {
    value.len() <= max && !value.chars().any(char::is_control)
}

fn valid_stream(stream: &str) -> bool {
    bounded(stream, MAX_STREAM_CHARS)
        && stream
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_capabilities(payload: &LivePayload, capabilities: &Capabilities) -> bool {
    match payload {
        LivePayload::Chat { .. } => capabilities.chat,
        LivePayload::Follow => capabilities.follows,
        LivePayload::Subscription { .. } => capabilities.subscriptions,
        LivePayload::Gift { .. } => capabilities.gifts,
        LivePayload::Like { .. } => capabilities.likes,
        LivePayload::Share { .. } => capabilities.shares,
        LivePayload::Join => capabilities.joins,
        LivePayload::RoomStats { .. } => capabilities.room_stats,
        LivePayload::Connected | LivePayload::Disconnected => true,
    }
}

fn validate_payload(payload: &LivePayload) -> Result<(), String> {
    match payload {
        LivePayload::Chat { comment } if comment.len() > MAX_TEXT_CHARS => {
            Err("chat comment is too long".into())
        }
        LivePayload::Subscription { tier, .. }
            if tier
                .as_deref()
                .is_some_and(|tier| !optional_bounded(tier, 64)) =>
        {
            Err("invalid subscription tier".into())
        }
        LivePayload::Gift {
            id,
            name,
            count,
            unit,
            amount,
        } => {
            if !bounded(id, 128) || !bounded(name, 128) || !bounded(unit, 64) {
                return Err("invalid gift identity".into());
            }
            if *count == 0 || *count > 1_000_000 {
                return Err("invalid gift count".into());
            }
            if !bounded(amount, 64)
                || !amount
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
            {
                return Err("invalid monetary amount".into());
            }
            Ok(())
        }
        LivePayload::Like { count, .. } | LivePayload::Share { count } => {
            if *count == 0 || *count > 1_000_000 {
                Err("invalid event count".into())
            } else {
                Ok(())
            }
        }
        LivePayload::RoomStats {
            viewers,
            total_users,
        } => {
            if total_users.is_some_and(|total| total < *viewers) {
                Err("room total cannot be below current viewers".into())
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

fn validate_user(user: &tiktools_live::ViewerInput) -> Result<(), String> {
    if !bounded(&user.id, MAX_ID_CHARS) {
        return Err("invalid platform user id".into());
    }
    if !optional_bounded(&user.handle, MAX_NAME_CHARS)
        || !optional_bounded(&user.display_name, MAX_NAME_CHARS)
    {
        return Err("invalid platform user display identity".into());
    }
    if let Some(url) = &user.avatar_url {
        if url.len() > MAX_URL_CHARS
            || url.chars().any(char::is_control)
            || !(url.starts_with("https://") || url.starts_with("http://"))
        {
            return Err("invalid avatar URL".into());
        }
    }
    Ok(())
}

fn validate_channel(channel: &tiktools_live::ChannelInput) -> Result<(), String> {
    if !bounded(&channel.id, MAX_ID_CHARS)
        || !bounded(&channel.handle, MAX_NAME_CHARS)
        || !optional_bounded(&channel.room_id, MAX_ID_CHARS)
    {
        return Err("invalid platform channel identity".into());
    }
    Ok(())
}

fn validate_error(error: &PlatformError) -> Result<(), String> {
    if bounded(&error.code, 64) && optional_bounded(&error.message, 512) {
        Ok(())
    } else {
        Err("invalid platform error".into())
    }
}

fn validate_event_input(input: &LiveEventInput) -> Result<(), String> {
    if input
        .source_event_id
        .as_deref()
        .is_some_and(|id| !optional_bounded(id, 256))
    {
        return Err("invalid source event id".into());
    }
    if let Some(user) = &input.user {
        validate_user(user)?;
    }
    validate_payload(&input.payload)?;
    if input.extensions.len() > 16
        || input
            .extensions
            .keys()
            .any(|key| !bounded(key, 64) || key.starts_with("host") || key.starts_with("tiktools"))
    {
        return Err("invalid platform extension metadata".into());
    }
    Ok(())
}

fn automation_data(event: &LiveEnvelope) -> Value {
    let mut data = match &event.payload {
        LivePayload::Chat { comment } => json!({
            "comment": comment,
            "emotes": [],
        }),
        LivePayload::Follow => json!({"action":1,"followCount":0,"shareCount":0}),
        LivePayload::Subscription {
            tier,
            months,
            gifted,
        } => json!({
            "action":3,
            "memberCount":0,
            "tier":tier,
            "months":months,
            "gifted":gifted,
        }),
        LivePayload::Gift {
            id,
            name,
            count,
            unit,
            amount,
        } => json!({
            "giftId":id,
            "giftName":name,
            "diamondCount":0,
            "repeatCount":count,
            "comboCount":count,
            "groupId":"",
            "repeatEnd":true,
            "streakable":false,
            "unit":unit,
            "amount":amount,
        }),
        LivePayload::Like { count, total } => json!({
            "count":count,
            "total":total.unwrap_or(*count),
        }),
        LivePayload::Share { count } => json!({"action":2,"followCount":0,"shareCount":count}),
        LivePayload::Join => json!({"action":0,"memberCount":0}),
        LivePayload::RoomStats {
            viewers,
            total_users,
        } => json!({
            "viewers":viewers,
            "totalUsers":total_users.unwrap_or(*viewers),
            "popularity":0,
            "anonymous":0,
        }),
        LivePayload::Connected | LivePayload::Disconnected => json!({
            "uniqueId": identity_key(&event.platform, &event.channel.id),
            "roomId": event.channel.room_id,
        }),
    };
    if let Some(object) = data.as_object_mut() {
        object.insert("method".into(), json!(event.payload.event_type()));
        object.insert(
            "msgId".into(),
            json!(event.source_event_id.as_deref().unwrap_or("")),
        );
        object.insert("isHistory".into(), json!(event.is_history));
    }
    data
}

fn ui_event(event: &LiveEnvelope, correlation: &str) -> Option<Value> {
    let user = event.user.as_ref()?;
    let (kind, text, key, params) = match &event.payload {
        LivePayload::Chat { comment } => (
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
            "subscribed".to_owned(),
            "platformSubscribed",
            json!({}),
        ),
        LivePayload::Follow => (
            "social",
            "followed the creator".to_owned(),
            "followedCreator",
            json!({}),
        ),
        LivePayload::Share { .. } => (
            "social",
            "shared the live".to_owned(),
            "sharedLive",
            json!({}),
        ),
        LivePayload::Join => (
            "member",
            "joined the live".to_owned(),
            "joinedLive",
            json!({}),
        ),
        LivePayload::Like { count, .. } => (
            "like",
            format!("sent {count} likes"),
            "likeSent",
            json!({"count":count}),
        ),
        _ => return None,
    };
    let mut ui = json!({
        "kind":kind,
        "author":user.key(),
        "nickname":user.display_name,
        "avatarUrl":user.avatar_url,
        "text":text,
        "platform":event.platform,
        "sourceType":event.payload.event_type(),
        "msgId":event.source_event_id,
        "roomId":event.channel.room_id,
        "creatorId":identity_key(&event.platform,&event.channel.id),
        "correlationId":correlation,
        "isHistory":event.is_history,
        "i18nKey":key,
        "i18nParams":params,
    });
    if let LivePayload::Gift {
        id,
        name,
        count,
        unit,
        amount,
    } = &event.payload
    {
        ui["giftDetails"] = json!({
            "giftId":id,
            "name":name,
            "count":count,
            "diamonds":0,
            "unit":unit,
            "amount":amount,
            "streakable":false,
        });
    }
    if let LivePayload::Like { count, .. } = &event.payload {
        ui["likeCount"] = json!(count);
        ui["likeViewerKey"] = json!(user.key());
    }
    Some(ui)
}

fn automation_envelope(event: &LiveEnvelope, event_id: &str) -> Value {
    let topic = event.payload.event_type();
    let data = automation_data(event);
    let user = event.user.as_ref().map(|user| {
        json!({
            "userId":user.id,
            "uniqueId":user.key(),
            "nickname":user.display_name,
            "avatarUrl":user.avatar_url,
            "secUid":"",
        })
    });
    let mut automation = json!({
        "id":event_id,
        "type":topic,
        "timestamp":event.occurred_at,
        "connectionId":event.connection_id,
        "creator":{
            "uniqueId":identity_key(&event.platform,&event.channel.id),
            "roomId":event.channel.room_id,
        },
        "user":user,
        "data":data,
        "platform":event.platform,
        "sourceEventId":event.source_event_id,
        "extensions":event.extensions,
    });
    if let Some(fields) = automation.as_object_mut() {
        // Host-owned provenance is explicit; plugin-controlled extension
        // data can never overwrite these top-level fields because they are
        // written after deserialization into a fresh envelope.
        fields.insert("platform".into(), json!(event.platform));
    }
    automation
}

fn system_envelope(
    session: &PlatformSessionSnapshot,
    payload: LivePayload,
) -> Option<LiveEnvelope> {
    let now = tiktools_live::now_millis();
    Some(LiveEnvelope {
        schema_version: SCHEMA_VERSION,
        platform: session.platform.clone(),
        connection_id: session.connection_id.clone(),
        session_id: session.session_id.clone(),
        channel: session.channel.clone()?,
        source_event_id: None,
        occurred_at: now,
        received_at: now,
        is_history: false,
        user: None,
        payload,
        extensions: Map::new(),
    })
}

fn dispatch_provider_disconnect(owner: &Arc<AppCore>, event: AutomationEvent) {
    if tokio::runtime::Handle::try_current().is_ok() {
        owner.queue_automation_event(event);
        return;
    }
    // Control-plane mutations are synchronous APIs and some embedders invoke
    // them outside a Tokio context. Keep disconnect semantics intact there
    // without panicking in `tokio::spawn`; this fallback is cold-path only.
    let owner = Arc::clone(owner);
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(owner.publish_automation_event(event));
    });
}

impl AppCore {
    pub(crate) async fn ingest_live_platform_batch(
        self: &Arc<Self>,
        plugin_id: &str,
        data: Value,
    ) -> Result<LiveIngestOutcome, String> {
        self.live_platforms.set_owner(self);
        let Some(plugin) = self.plugins.get(plugin_id) else {
            return Err("plugin is not in the host catalog".into());
        };
        self.capabilities
            .require_capability(&plugin.manifest, LIVE_INGEST_CAPABILITY)
            .map_err(|error| error.to_string())?;
        let batch: LiveIngestBatch = serde_json::from_value(data)
            .map_err(|error| format!("invalid live ingest batch: {error}"))?;
        if batch.schema_version != SCHEMA_VERSION {
            return Err("unsupported live ingest schema version".into());
        }
        if batch.messages.is_empty() || batch.messages.len() > MAX_BATCH_MESSAGES {
            return Err("invalid live ingest batch size".into());
        }
        let mut outcome = LiveIngestOutcome::default();
        for message in batch.messages {
            match self.ingest_live_platform_message(plugin_id, message).await {
                Ok(true) => outcome.accepted += 1,
                Ok(false) => outcome.duplicates += 1,
                Err(error) => {
                    outcome.rejected += 1;
                    tracing::warn!(plugin = %plugin_id, %error, "live platform ingest message rejected");
                }
            }
        }
        Ok(outcome)
    }

    async fn ingest_live_platform_message(
        self: &Arc<Self>,
        plugin_id: &str,
        message: PlatformIngestMessage,
    ) -> Result<bool, String> {
        match message {
            PlatformIngestMessage::SessionStart {
                stream,
                channel,
                capabilities,
            } => {
                if !valid_stream(&stream) {
                    return Err("invalid stream key".into());
                }
                validate_channel(&channel)?;
                let now = tiktools_live::now_millis();
                let snapshot = {
                    let mut sessions =
                        recover_mutex(&self.live_platforms.sessions, "live platform sessions");
                    let plugin_count = sessions
                        .keys()
                        .filter(|(plugin, _)| plugin == plugin_id)
                        .count();
                    let key = (plugin_id.to_owned(), stream.clone());
                    if !sessions.contains_key(&key)
                        && (sessions.len() >= MAX_SESSIONS_TOTAL
                            || plugin_count >= MAX_SESSIONS_PER_PLUGIN)
                    {
                        return Err("platform session capacity reached".into());
                    }
                    let room_id = if channel.room_id.is_empty() {
                        channel.id.clone()
                    } else {
                        channel.room_id
                    };
                    let channel = ChannelIdentity {
                        platform: plugin_id.to_owned(),
                        id: channel.id,
                        handle: channel.handle,
                        room_id,
                    };
                    let entry = sessions.entry(key).or_insert_with(|| {
                        let sequence = self.next_sequence();
                        PlatformSession {
                            snapshot: PlatformSessionSnapshot {
                                session_id: format!("platform-session-{sequence}"),
                                connection_id: format!("platform-connection-{sequence}"),
                                platform: plugin_id.to_owned(),
                                stream: stream.clone(),
                                state: SessionState::Connecting,
                                channel: None,
                                capabilities: capabilities.clone(),
                                reconnect_attempt: 0,
                                retry_after_ms: None,
                                error: None,
                                started_at: now,
                                updated_at: now,
                            },
                            ended: false,
                        }
                    });
                    entry.ended = false;
                    entry.snapshot.channel = Some(channel);
                    entry.snapshot.capabilities = capabilities;
                    entry.snapshot.state = SessionState::Connected;
                    entry.snapshot.error = None;
                    entry.snapshot.retry_after_ms = None;
                    entry.snapshot.updated_at = now;
                    entry.snapshot.clone()
                };
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformSession {
                        session: snapshot.clone(),
                    });
                if let Some(input) = system_event_input(LivePayload::Connected) {
                    self.ingest_session_event(plugin_id, &snapshot.stream, input)
                        .await?;
                }
                Ok(true)
            }
            PlatformIngestMessage::SessionState {
                stream,
                state,
                reconnect_attempt,
                retry_after_ms,
                error,
            } => {
                if !valid_stream(&stream) {
                    return Err("invalid stream key".into());
                }
                if let Some(error) = &error {
                    validate_error(error)?;
                }
                let snapshot = {
                    let mut sessions =
                        recover_mutex(&self.live_platforms.sessions, "live platform sessions");
                    let session = sessions
                        .get_mut(&(plugin_id.to_owned(), stream.clone()))
                        .ok_or_else(|| "platform session does not exist".to_owned())?;
                    session.snapshot.state = state;
                    session.snapshot.reconnect_attempt = reconnect_attempt;
                    session.snapshot.retry_after_ms = retry_after_ms.filter(|ms| *ms <= 3_600_000);
                    session.snapshot.error = error;
                    session.snapshot.updated_at = tiktools_live::now_millis();
                    session.snapshot.clone()
                };
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformSession { session: snapshot });
                Ok(true)
            }
            PlatformIngestMessage::SessionEnd { stream, error } => {
                if !valid_stream(&stream) {
                    return Err("invalid stream key".into());
                }
                if let Some(error) = &error {
                    validate_error(error)?;
                }
                let snapshot = {
                    let mut sessions =
                        recover_mutex(&self.live_platforms.sessions, "live platform sessions");
                    let session = sessions
                        .get_mut(&(plugin_id.to_owned(), stream.clone()))
                        .ok_or_else(|| "platform session does not exist".to_owned())?;
                    session.ended = true;
                    session.snapshot.state = if error.is_some() {
                        SessionState::Error
                    } else {
                        SessionState::Disconnected
                    };
                    session.snapshot.error = error;
                    session.snapshot.updated_at = tiktools_live::now_millis();
                    session.snapshot.clone()
                };
                if let Some(input) = system_event_input(LivePayload::Disconnected) {
                    let _ = self.ingest_session_event(plugin_id, &stream, input).await;
                }
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformSession { session: snapshot });
                recover_mutex(&self.live_platforms.sessions, "live platform sessions")
                    .remove(&(plugin_id.to_owned(), stream));
                Ok(true)
            }
            PlatformIngestMessage::Gap { stream, lost } => {
                if !valid_stream(&stream) || lost == 0 || lost > 1_000_000 {
                    return Err("invalid platform gap".into());
                }
                let connection_id = {
                    let sessions =
                        recover_mutex(&self.live_platforms.sessions, "live platform sessions");
                    sessions
                        .get(&(plugin_id.to_owned(), stream))
                        .ok_or_else(|| "platform session does not exist".to_owned())?
                        .snapshot
                        .connection_id
                        .clone()
                };
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformGap {
                        connection_id,
                        lost,
                    });
                Ok(true)
            }
            PlatformIngestMessage::Event { stream, event } => {
                if !valid_stream(&stream) {
                    return Err("invalid stream key".into());
                }
                self.ingest_session_event(plugin_id, &stream, event).await
            }
        }
    }

    async fn ingest_session_event(
        self: &Arc<Self>,
        plugin_id: &str,
        stream: &str,
        input: LiveEventInput,
    ) -> Result<bool, String> {
        validate_event_input(&input)?;
        let session = {
            let sessions = recover_mutex(&self.live_platforms.sessions, "live platform sessions");
            sessions
                .get(&(plugin_id.to_owned(), stream.to_owned()))
                .filter(|session| {
                    !session.ended || matches!(input.payload, LivePayload::Disconnected)
                })
                .map(|session| session.snapshot.clone())
        }
        .ok_or_else(|| "platform session does not exist".to_owned())?;
        if session.state != SessionState::Connected
            && !matches!(
                input.payload,
                LivePayload::Connected | LivePayload::Disconnected
            )
        {
            return Err("platform session is not connected".into());
        }
        if !valid_capabilities(&input.payload, &session.capabilities) {
            return Err("platform capability was not declared by the session".into());
        }
        let channel = session
            .channel
            .clone()
            .ok_or_else(|| "platform session has no verified channel".to_owned())?;
        let received_at = tiktools_live::now_millis();
        let envelope = LiveEnvelope {
            schema_version: SCHEMA_VERSION,
            platform: plugin_id.to_owned(),
            connection_id: session.connection_id.clone(),
            session_id: session.session_id.clone(),
            channel,
            source_event_id: input.source_event_id,
            occurred_at: input.occurred_at.unwrap_or(received_at).min(received_at),
            received_at,
            is_history: input.is_history,
            user: input.user.map(|user| {
                let display_name = if user.display_name.is_empty() {
                    user.handle.clone()
                } else {
                    user.display_name
                };
                ViewerIdentity {
                    platform: plugin_id.to_owned(),
                    id: user.id,
                    handle: user.handle,
                    display_name,
                    avatar_url: user.avatar_url,
                }
            }),
            payload: input.payload,
            extensions: input.extensions,
        };
        if self.live_platforms.duplicate(&envelope) {
            return Ok(false);
        }
        self.handle_live_platform_event(envelope).await?;
        Ok(true)
    }

    async fn handle_live_platform_event(
        self: &Arc<Self>,
        event: LiveEnvelope,
    ) -> Result<(), String> {
        self.events
            .publish_domain(events::DomainEvent::LivePlatformEvent {
                event: event.clone(),
            });
        let topic = event.payload.event_type();
        let event_id = format!("platform-{}-{}", event.connection_id, self.next_sequence());
        let automation = automation_envelope(&event, &event_id);
        if event.is_history {
            self.events.publish_domain(events::DomainEvent::LiveEvent {
                event_type: topic.into(),
                event: automation,
            });
        } else {
            self.queue_automation_event(AutomationEvent::owner_without_delegation(automation));
        }
        if let Some(ui) = ui_event(&event, &event_id) {
            let creator = identity_key(&event.platform, &event.channel.id);
            self.events
                .publish_live_event_from_creator(ui, Some(&creator));
        }
        if event.is_history {
            return Ok(());
        }
        #[cfg(feature = "persistence")]
        {
            let db = self.db.clone();
            let record = event.clone();
            let recorded =
                tokio::task::spawn_blocking(move || db.record_platform_activity(&record))
                    .await
                    .map_err(|_| "platform analytics task failed".to_owned())?;
            if recorded.is_err() {
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformGap {
                        connection_id: event.connection_id.clone(),
                        lost: 1,
                    });
                return Err("platform analytics persistence failed".into());
            }
        }
        let Some(user) = event.user else {
            return Ok(());
        };
        let action = match event.payload {
            LivePayload::Chat { .. } => PointAction::Chat,
            LivePayload::Follow => PointAction::Follow,
            LivePayload::Like { .. } => PointAction::Like,
            LivePayload::Share { .. } => PointAction::Share,
            LivePayload::Join => PointAction::Join,
            _ => return Ok(()),
        };
        let count = match event.payload {
            LivePayload::Like { count, .. } | LivePayload::Share { count } => Some(count as f64),
            _ => None,
        };
        let key = user.key();
        let options = AwardOptions {
            user_id: Some(user.id.clone()),
            nickname: Some(user.display_name.clone()),
            avatar_url: user.avatar_url.clone(),
            count,
            ..Default::default()
        };
        let points = self.points.clone();
        match tokio::task::spawn_blocking(move || points.award_points(&key, action, options)).await
        {
            Ok(Some(award)) => {
                if award.delta != 0.0 {
                    let data =
                        serde_json::to_value(crate::contracts::PointsAwardedAutomationData {
                            unique_id: award.unique_id.clone(),
                            delta: award.delta,
                            total_points: award.total_points,
                            level: award.level,
                            currency_name: award.currency_name.clone(),
                            reason: topic.trim_start_matches("live.").into(),
                        })
                        .expect("points awarded data");
                    self.queue_automation_event(AutomationEvent::owner_without_delegation(json!({
                        "id":format!("points-awarded-{event_id}"),
                        "type":"points.awarded",
                        "platform":event.platform,
                        "timestamp":tiktools_live::now_millis(),
                        "connectionId":event.connection_id,
                        "creator":{"uniqueId":identity_key(&event.platform,&event.channel.id),"roomId":event.channel.room_id},
                        "data":data,
                    })));
                }
                self.events
                    .publish_domain(events::DomainEvent::PointsChanged {
                        unique_id: award.unique_id,
                        delta: award.delta,
                        total_points: award.total_points,
                        level: award.level,
                    });
                self.emit_leaderboard_if_due();
            }
            Ok(None) => {}
            Err(_) => {
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformGap {
                        connection_id: event.connection_id,
                        lost: 1,
                    });
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn live_platform_sessions(&self) -> Vec<PlatformSessionSnapshot> {
        recover_mutex(&self.live_platforms.sessions, "live platform sessions")
            .values()
            .filter(|session| !session.ended)
            .map(|session| session.snapshot.clone())
            .collect()
    }

    pub(crate) fn end_live_platform_plugin_sessions(&self, plugin_id: &str) {
        for session in self.live_platforms.end_plugin_sessions(plugin_id) {
            let disconnected = system_envelope(&session, LivePayload::Disconnected);
            self.events
                .publish_domain(events::DomainEvent::LivePlatformSession { session });
            let Some(event) = disconnected else {
                continue;
            };
            self.events
                .publish_domain(events::DomainEvent::LivePlatformEvent {
                    event: event.clone(),
                });
            let event_id = format!("platform-{}-{}", event.connection_id, self.next_sequence());
            if let Some(owner) = self.live_platforms.owner() {
                dispatch_provider_disconnect(
                    &owner,
                    AutomationEvent::owner_without_delegation(automation_envelope(
                        &event, &event_id,
                    )),
                );
            }
            #[cfg(feature = "persistence")]
            if self.db.record_platform_activity(&event).is_err() {
                self.events
                    .publish_domain(events::DomainEvent::LivePlatformGap {
                        connection_id: event.connection_id,
                        lost: 1,
                    });
            }
        }
    }
}

fn system_event_input(payload: LivePayload) -> Option<LiveEventInput> {
    Some(LiveEventInput {
        source_event_id: None,
        occurred_at: None,
        is_history: false,
        user: None,
        payload,
        extensions: Map::new(),
    })
}

#[cfg(test)]
mod tests;
