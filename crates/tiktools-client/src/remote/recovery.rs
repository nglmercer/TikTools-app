//! Recovery never retries domain RPCs or discards a caller's local drafts.
use super::*;
use crate::{ControlEvent, TikToolsClient};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::{collections::BTreeSet, time::Duration};
#[cfg(feature = "remote-profiles")]
use tiktools_collaboration::ResumeResult;
use tiktools_collaboration::{
    stream_sequence, ResourceKind, SnapshotPage, StreamDomain, SubscribeResult, SNAPSHOT_BYTES,
};

// The gateway rejects rate-limited requests before dispatch. Only that
// explicit response permits retry: an ambiguous transport failure might have
// created a subscription already and must never be replayed here.
async fn retry_recovery_read<T, F, Fut>(mut call: F) -> Result<T, ClientError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ClientError>>,
{
    for attempt in 0..=4 {
        match call().await {
            Err(error) if error.code == "rate_limited" && attempt < 4 => {
                let millis = error
                    .data
                    .as_ref()
                    .and_then(|data| data.get("retryAfterMs"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(1000)
                    .clamp(1000, 2000);
                tokio::time::sleep(Duration::from_millis(millis)).await;
            }
            result => return result,
        }
    }
    unreachable!()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryCursor {
    pub subscription_id: String,
    pub host_instance_id: String,
    pub session_id: String,
    pub grant_revision: String,
    pub last_applied_sequence: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    Resumed,
    Snapshot,
}
pub struct RecoveryStream {
    remote: RemoteClient,
    receiver: tokio::sync::broadcast::Receiver<ControlEvent>,
    domains: BTreeSet<StreamDomain>,
    points_limit: Option<u16>,
    cursor: Option<RecoveryCursor>,
    baseline: Option<SnapshotPage>,
    ready: bool,
    pending: Option<ReliableEvent>,
}
impl RecoveryStream {
    pub async fn start(
        remote: RemoteClient,
        domains: BTreeSet<StreamDomain>,
    ) -> Result<Self, ClientError> {
        Self::start_projected(remote, domains, None).await
    }
    pub async fn start_projected(
        remote: RemoteClient,
        domains: BTreeSet<StreamDomain>,
        points_limit: Option<u16>,
    ) -> Result<Self, ClientError> {
        if points_limit.is_some_and(|n| n > 1000 || !domains.contains(&StreamDomain::Points)) {
            return Err(fail("invalid_params", "invalid points projection"));
        }
        if points_limit.is_some() && remote.hello().features.get("pointsProjection") != Some(&true)
        {
            return Err(fail(
                "unsupported_remote",
                "bounded points projection is unavailable",
            ));
        }
        if domains.contains(&StreamDomain::Automation)
            && remote.hello().features.get("automationSummary") != Some(&true)
        {
            return Err(fail(
                "unsupported_remote",
                "automation status projection is unavailable",
            ));
        }
        if domains.is_empty()
            || remote.hello().features.get("subscriptions") != Some(&true)
            || domains
                .iter()
                .any(|d| !remote.hello().subscription_domains.contains(d))
        {
            return Err(fail(
                "unsupported_remote",
                "requested remote recovery domains are unavailable",
            ));
        }
        // Subscribe locally before any RPC so early post-baseline events are
        // retained even if the reader wins the last-page response wakeup race.
        let receiver = remote.subscribe();
        let sdk = TikToolsClient::Remote(remote.clone());
        let sub: SubscribeResult = retry_recovery_read(|| {
            sdk.collaboration_subscribe(SubscribeParams {
                domains: domains.clone(),
                feed: false,
                points_limit,
            })
        })
        .await?;
        if sub.host_instance_id != remote.hello().host_instance_id
            || sub.grant_revision != remote.hello().grant_revision
            || sub.domains != domains
            || sub.points_limit != points_limit
            || !valid_id(&sub.subscription_id)
        {
            return Err(protocol());
        }
        let mut page = retry_recovery_read(|| {
            sdk.collaboration_snapshot(SnapshotParams {
                subscription_id: sub.subscription_id.clone(),
                page_token: None,
            })
        })
        .await?;
        let mut baseline = page.clone();
        baseline.workflows.clear();
        baseline.points.clear();
        baseline.automation.clear();
        baseline.resource_versions.clear();
        baseline.next_page_token = None;
        let mut tokens = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut bytes = 0usize;
        loop {
            if page.subscription_id != sub.subscription_id
                || page.host_instance_id != sub.host_instance_id
                || page.grant_revision != sub.grant_revision
                || page.snapshot_id != baseline.snapshot_id
                || page.through_sequence != baseline.through_sequence
                || page.domains != domains
                || page.points_limit != points_limit
                || !valid_id(&page.snapshot_id)
                || stream_sequence(&page.through_sequence).is_err()
                || page.workflows.len() + page.points.len() + page.automation.len()
                    != page.resource_versions.len()
            {
                return Err(protocol());
            }
            bytes = bytes
                .checked_add(serde_json::to_vec(&page).map_err(|_| protocol())?.len())
                .ok_or_else(protocol)?;
            if bytes > SNAPSHOT_BYTES {
                return Err(fail(
                    "too_large",
                    "remote baseline exceeds the recovery budget",
                ));
            }
            validate_page_records(&page, &domains, &mut ids)?;
            baseline.workflows.append(&mut page.workflows);
            baseline.points.append(&mut page.points);
            baseline.automation.append(&mut page.automation);
            baseline
                .resource_versions
                .append(&mut page.resource_versions);
            let Some(token) = page.next_page_token else {
                break;
            };
            if !tokens.insert(token.clone()) || tokens.len() > 4096 {
                return Err(protocol());
            }
            page = retry_recovery_read(|| {
                sdk.collaboration_snapshot(SnapshotParams {
                    subscription_id: sub.subscription_id.clone(),
                    page_token: Some(token.clone()),
                })
            })
            .await?;
        }
        if points_limit.is_some_and(|n| baseline.points.len() > usize::from(n)) {
            return Err(protocol());
        }
        let cursor = RecoveryCursor {
            subscription_id: sub.subscription_id,
            host_instance_id: sub.host_instance_id,
            session_id: remote.hello().session_id.clone(),
            grant_revision: sub.grant_revision,
            last_applied_sequence: baseline.through_sequence.clone(),
        };
        Ok(Self {
            remote,
            receiver,
            domains,
            points_limit,
            cursor: Some(cursor),
            baseline: Some(baseline),
            ready: true,
            pending: None,
        })
    }
    pub fn is_ready(&self) -> bool {
        self.ready && !self.remote.is_closed() && !self.remote.requires_resync()
    }
    pub fn baseline(&self) -> Option<&SnapshotPage> {
        if self.is_ready() {
            self.baseline.as_ref()
        } else {
            None
        }
    }
    pub fn cursor(&self) -> Option<&RecoveryCursor> {
        self.cursor.as_ref()
    }
    pub fn domains(&self) -> &BTreeSet<StreamDomain> {
        &self.domains
    }
    pub fn connection(&self) -> &RemoteClient {
        &self.remote
    }
    fn invalidate(&mut self) {
        self.ready = false;
        self.pending = None;
        self.baseline = None;
        self.cursor = None;
    }
    pub async fn next_event(&mut self) -> Result<ReliableEvent, ClientError> {
        if self.pending.is_some() {
            return Err(fail(
                "ack_required",
                "apply and acknowledge the prior reliable event first",
            ));
        }
        loop {
            if self.remote.requires_resync() {
                self.invalidate();
                return Err(fail(
                    "resync_required",
                    "remote stream requires a new baseline",
                ));
            }
            if self.remote.is_closed() {
                self.ready = false;
                return Err(self.remote.closed_reason().unwrap_or_else(network));
            }
            let event = match self.receiver.recv().await {
                Ok(ControlEvent::Reliable(event)) => event,
                Ok(ControlEvent::Gap { .. }) => {
                    if self.remote.requires_resync() {
                        self.invalidate();
                    } else {
                        self.ready = false;
                    }
                    return Err(fail(
                        "resync_required",
                        "remote event delivery was interrupted",
                    ));
                }
                Ok(ControlEvent::Domain(_)) => {
                    self.invalidate();
                    return Err(protocol());
                }
                Err(_) => {
                    self.ready = false;
                    return Err(fail(
                        "resync_required",
                        "local reliable receiver lost its recovery position",
                    ));
                }
            };
            let cursor = self
                .cursor
                .as_ref()
                .ok_or_else(|| fail("resync_required", "a complete baseline is required"))?;
            if event.subscription_id != cursor.subscription_id
                || event.host_instance_id != cursor.host_instance_id
                || !valid_event(&event, &self.domains)
            {
                self.invalidate();
                return Err(protocol());
            }
            let sequence = stream_sequence(&event.sequence).map_err(|_| protocol())?;
            let applied = stream_sequence(&cursor.last_applied_sequence).map_err(|_| protocol())?;
            if sequence <= applied {
                continue;
            }
            if sequence != applied.saturating_add(1) {
                self.ready = false;
                return Err(fail("resync_required", "reliable sequence has a gap"));
            }
            self.pending = Some(event.clone());
            return Ok(event);
        }
    }
    /// Advance only after the caller has applied the event to its own state.
    /// A lost connection with an unacknowledged event replays that event.
    pub fn acknowledge(&mut self, event: &ReliableEvent) -> Result<(), ClientError> {
        if self.pending.as_ref() != Some(event) {
            return Err(fail(
                "invalid_params",
                "event does not match the pending reliable update",
            ));
        }
        self.cursor
            .as_mut()
            .ok_or_else(protocol)?
            .last_applied_sequence = event.sequence.clone();
        self.pending = None;
        Ok(())
    }
    #[cfg(feature = "remote-profiles")]
    pub async fn reconnect(&mut self) -> Result<RecoveryMode, ClientError> {
        self.ready = false;
        self.pending = None;
        if self.remote.requires_resync() {
            self.invalidate();
        }
        self.remote.close();
        let remote = self.remote.reopen().await?;
        if let Some(cursor) = self.cursor.clone().filter(|c| {
            c.host_instance_id == remote.hello().host_instance_id
                && c.session_id == remote.hello().session_id
                && c.grant_revision == remote.hello().grant_revision
        }) {
            let receiver = remote.subscribe();
            let sdk = TikToolsClient::Remote(remote.clone());
            let resumed: Result<ResumeResult, _> = retry_recovery_read(|| {
                sdk.collaboration_resume(ResumeParams {
                    subscription_id: cursor.subscription_id.clone(),
                    host_instance_id: cursor.host_instance_id.clone(),
                    last_applied_sequence: cursor.last_applied_sequence.clone(),
                })
            })
            .await;
            match resumed {
                Ok(reply) => {
                    if reply.subscription_id != cursor.subscription_id
                        || reply.host_instance_id != cursor.host_instance_id
                        || reply.grant_revision != cursor.grant_revision
                        || reply.after_sequence != cursor.last_applied_sequence
                        || stream_sequence(&reply.through_sequence).map_or(true, |n| {
                            n < stream_sequence(&reply.after_sequence).unwrap_or(u64::MAX)
                        })
                    {
                        self.invalidate();
                        return Err(protocol());
                    }
                    self.remote = remote;
                    self.receiver = receiver;
                    self.ready = true;
                    return Ok(RecoveryMode::Resumed);
                }
                Err(error) if error.code == "resync_required" => {}
                Err(error) => return Err(error),
            }
        }
        self.invalidate();
        let fresh = Self::start_projected(remote, self.domains.clone(), self.points_limit).await?;
        *self = fresh;
        Ok(RecoveryMode::Snapshot)
    }
}

fn validate_page_records(
    page: &SnapshotPage,
    domains: &BTreeSet<StreamDomain>,
    ids: &mut BTreeSet<(ResourceKind, String, String)>,
) -> Result<(), ClientError> {
    let mut bodies = std::collections::BTreeMap::new();
    for (kind, domain, field, rows) in [
        (
            ResourceKind::Workflow,
            StreamDomain::Workflows,
            "id",
            &page.workflows,
        ),
        (
            ResourceKind::Viewer,
            StreamDomain::Points,
            "uniqueId",
            &page.points,
        ),
    ] {
        if !rows.is_empty() && !domains.contains(&domain) {
            return Err(protocol());
        }
        for record in rows {
            let id = record
                .get(field)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(protocol)?;
            if bodies.insert((kind, "host", id), ()).is_some() {
                return Err(protocol());
            }
        }
    }
    if !page.automation.is_empty() && !domains.contains(&StreamDomain::Automation) {
        return Err(protocol());
    }
    for record in &page.automation {
        let summary: tiktools_collaboration::AutomationSummary =
            serde_json::from_value(record.clone()).map_err(|_| protocol())?;
        if !summary.definition_redacted
            || summary.name.chars().count() > 160
            || !tiktools_collaboration::Grant::default()
                .allows_automation(summary.kind, &summary.id)
        {
            return Err(protocol());
        }
        let id = record
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(protocol)?;
        let scope = record
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(protocol)?;
        if bodies
            .insert((ResourceKind::Automation, scope, id), ())
            .is_some()
        {
            return Err(protocol());
        }
    }
    for version in &page.resource_versions {
        if !resource_scope_valid(version.kind, &version.scope)
            || bodies
                .remove(&(version.kind, version.scope.as_str(), version.id.as_str()))
                .is_none()
            || stream_sequence(&version.revision).map_or(true, |n| n == 0)
            || !ids.insert((version.kind, version.scope.clone(), version.id.clone()))
        {
            return Err(protocol());
        }
    }
    if !bodies.is_empty() {
        return Err(protocol());
    }
    Ok(())
}

fn resource_scope_valid(kind: ResourceKind, scope: &str) -> bool {
    if kind == ResourceKind::Automation {
        tiktools_collaboration::AutomationFamily::from_scope(scope).is_some()
    } else {
        matches!(kind, ResourceKind::Workflow | ResourceKind::Viewer) && scope == "host"
    }
}
fn valid_event(event: &ReliableEvent, domains: &BTreeSet<StreamDomain>) -> bool {
    if !resource_scope_valid(event.resource.kind, &event.resource.scope)
        || stream_sequence(&event.resource.revision)
            .map_or(true, |n| n == 0 && event.topic != "command.changed")
    {
        return false;
    }
    match event.topic.as_str() {
        "workflow.changed" => {
            event.resource.kind == ResourceKind::Workflow
                && domains.contains(&StreamDomain::Workflows)
                && event.data.get("id").and_then(serde_json::Value::as_str)
                    == Some(event.resource.id.as_str())
        }
        "points.changed" => {
            event.resource.kind == ResourceKind::Viewer
                && domains.contains(&StreamDomain::Points)
                && event.data.get("id").and_then(serde_json::Value::as_str)
                    == Some(event.resource.id.as_str())
        }
        "automation.changed" => {
            event.resource.kind == ResourceKind::Automation
                && domains.contains(&StreamDomain::Automation)
                && event.data.get("id").and_then(serde_json::Value::as_str)
                    == Some(event.resource.id.as_str())
                && event.data.get("kind").and_then(serde_json::Value::as_str)
                    == Some(event.resource.scope.as_str())
        }
        "command.changed" => {
            if !domains.contains(&StreamDomain::Commands)
                || !matches!(
                    event.resource.kind,
                    ResourceKind::Workflow | ResourceKind::Viewer | ResourceKind::Automation
                )
            {
                return false;
            }
            let Some(command) = event.data.get("command").cloned().and_then(|v| {
                serde_json::from_value::<tiktools_collaboration::CommandReceipt>(v).ok()
            }) else {
                return false;
            };
            command.resource.is_some_and(|resource| {
                resource.kind == event.resource.kind
                    && resource.scope == event.resource.scope
                    && resource.id == event.resource.id
            }) && event.command_id.as_ref() == Some(&command.command_id)
        }
        _ => false,
    }
}

#[derive(Default)]
pub struct ReconnectBackoff {
    attempt: u32,
}
impl ReconnectBackoff {
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
    pub fn next_delay(&mut self) -> Duration {
        let base = 500u64
            .saturating_mul(1u64 << self.attempt.min(6))
            .min(30_000);
        self.attempt = self.attempt.saturating_add(1);
        let mut byte = [0];
        let _ = getrandom::fill(&mut byte);
        Duration::from_millis((base + base * byte[0] as u64 / 1024).min(30_000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test(start_paused = true)]
    async fn recovery_retries_only_explicit_rate_limits_with_a_bounded_budget() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = AtomicUsize::new(0);
        let start = tokio::time::Instant::now();
        let error = retry_recovery_read(|| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err::<(), _>(fail("rate_limited", "limited")) }
        })
        .await
        .unwrap_err();
        assert_eq!(error.code, "rate_limited");
        assert_eq!(calls.load(Ordering::SeqCst), 5);
        assert_eq!(start.elapsed(), Duration::from_secs(4));

        for code in ["transport", "timeout", "forbidden"] {
            calls.store(0, Ordering::SeqCst);
            let error = retry_recovery_read(|| {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Err::<(), _>(fail(code, "failed")) }
            })
            .await
            .unwrap_err();
            assert_eq!(error.code, code);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn identical_ids_cannot_cross_stream_domains_or_command_targets() {
        let mut event: ReliableEvent = serde_json::from_value(json!({
            "topic":"points.changed","data":{"id":"same","change":"adjusted"},
            "hostInstanceId":"host","subscriptionId":"sub","sequence":"1","eventId":"event",
            "resource":{"kind":"viewer","scope":"host","id":"same","revision":"2"},
            "actor":{"principalId":"moderator"}
        }))
        .unwrap();
        let points = [StreamDomain::Points].into();
        assert!(valid_event(&event, &points));
        assert!(!valid_event(&event, &[StreamDomain::Workflows].into()));
        event.resource.kind = ResourceKind::Workflow;
        assert!(!valid_event(&event, &points));
        event.topic = "command.changed".into();
        event.command_id = Some("adjust".into());
        event.data = json!({"command":{"commandId":"adjust","state":"succeeded",
            "resource":{"kind":"viewer","scope":"host","id":"same"}}});
        let commands = [StreamDomain::Commands].into();
        assert!(!valid_event(&event, &commands));
        event.resource.kind = ResourceKind::Viewer;
        assert!(valid_event(&event, &commands));
        // A successful no-op delete of a never-created target has revision0.
        // It is an outcome receipt, not a live-row invalidation.
        event.resource.revision = "0".into();
        assert!(valid_event(&event, &commands));
        event.topic = "workflow.changed".into();
        assert!(!valid_event(&event, &[StreamDomain::Workflows].into()));
        event.topic = "command.changed".into();
        event.command_id = Some("different-intent".into());
        assert!(!valid_event(&event, &commands));
    }
}

#[cfg(test)]
mod automation_tests {
    use super::*;
    use serde_json::json;
    fn page() -> SnapshotPage {
        serde_json::from_value(json!({"subscriptionId":"sub","snapshotId":"snapshot","hostInstanceId":"instance","grantRevision":"1","throughSequence":"0","domains":["automation"],"workflows":[],"automation":[
            {"kind":"event","id":"same","name":"Event","enabled":true,"definitionRedacted":true},
            {"kind":"action","id":"same","name":"Action","enabled":false,"definitionRedacted":true}],
            "resourceVersions":[{"kind":"automation","scope":"event","id":"same","revision":"1"},{"kind":"automation","scope":"action","id":"same","revision":"3"}]})).unwrap()
    }
    #[test]
    fn automation_snapshot_validates_closed_summaries_and_independent_family_keys() {
        let original = page();
        let domains = [StreamDomain::Automation].into();
        assert!(validate_page_records(&original, &domains, &mut BTreeSet::new()).is_ok());
        for mutation in [
            "scope",
            "duplicate",
            "payload",
            "flag",
            "name",
            "id",
            "domain",
        ] {
            let mut p = page();
            match mutation {
                "scope" => p.resource_versions[1].scope = "host".into(),
                "duplicate" => p.automation[1]["kind"] = "event".into(),
                "payload" => p.automation[1]["config"] = json!({"secret":"forged"}),
                "flag" => p.automation[0]["definitionRedacted"] = false.into(),
                "name" => p.automation[0]["name"] = "x".repeat(161).into(),
                "id" => p.automation[0]["id"] = " spaced ".into(),
                "domain" => {}
                _ => unreachable!(),
            }
            let d = if mutation == "domain" {
                [StreamDomain::Workflows].into()
            } else {
                domains.clone()
            };
            assert!(
                validate_page_records(&p, &d, &mut BTreeSet::new()).is_err(),
                "{mutation}"
            );
        }
        let mut ids = BTreeSet::new();
        validate_page_records(&original, &domains, &mut ids).unwrap();
        assert!(
            validate_page_records(&original, &domains, &mut ids).is_err(),
            "duplicates across pages"
        );
    }
    #[test]
    fn automation_events_bind_summary_family_and_command_target_scope() {
        let mut event:ReliableEvent=serde_json::from_value(json!({"topic":"automation.changed","data":{"id":"same","kind":"event","change":"disabled"},"hostInstanceId":"host","subscriptionId":"sub","sequence":"1","eventId":"event","resource":{"kind":"automation","scope":"event","id":"same","revision":"2"},"actor":{"principalId":"moderator"}})).unwrap();
        assert!(valid_event(&event, &[StreamDomain::Automation].into()));
        assert!(!valid_event(&event, &[StreamDomain::Workflows].into()));
        event.resource.scope = "action".into();
        assert!(!valid_event(&event, &[StreamDomain::Automation].into()));
        event.topic = "command.changed".into();
        event.command_id = Some("stop".into());
        event.data = json!({"command":{"commandId":"stop","state":"succeeded","resource":{"kind":"automation","scope":"event","id":"same"}}});
        assert!(!valid_event(&event, &[StreamDomain::Commands].into()));
        event.resource.scope = "event".into();
        assert!(valid_event(&event, &[StreamDomain::Commands].into()));
    }
}
