use super::*;
use crate::{Authenticated, Capability, CoordinationEvent, Error, RemoteLimits, ResourceKind};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Default)]
pub struct StreamHub {
    pub(super) state: Mutex<State>,
}
#[derive(Default)]
pub(super) struct State {
    pub streams: BTreeMap<String, Stream>,
}
pub(super) struct Retained {
    pub event: ReliableEvent,
    pub size: usize,
    pub at: Instant,
}
pub(super) struct Stream {
    pub points_limit: Option<u16>,
    pub identity: Authenticated,
    pub connection: Option<String>,
    pub detached_at: Option<Instant>,
    pub host_instance: String,
    pub domains: BTreeSet<StreamDomain>,
    pub requests_per_second: u32,
    pub request_burst: u32,
    pub sequence: u64,
    pub sent: u64,
    pub events: VecDeque<Retained>,
    pub seen: BTreeSet<String>,
    pub bytes: usize,
    pub broken: bool,
    pub baseline_complete: bool,
    pub snapshot: Option<super::snapshots::Snapshot>,
}
impl Stream {
    pub fn matches(&self, identity: &Authenticated) -> bool {
        self.identity.principal_id == identity.principal_id
            && self.identity.session_id == identity.session_id
            && self.identity.grant_revision == identity.grant_revision
            && self.identity.grant == identity.grant
    }
    pub fn break_stream(&mut self) {
        self.broken = true;
        self.events.clear();
        self.seen.clear();
        self.bytes = 0;
        self.snapshot = None;
        self.baseline_complete = false;
    }
    fn trim(&mut self, now: Instant) {
        while self
            .events
            .front()
            .is_some_and(|e| now.duration_since(e.at) >= Duration::from_secs(STREAM_SECONDS))
            || self.events.len() > STREAM_EVENTS
            || self.bytes > STREAM_BYTES
        {
            let Some(old) = self.events.pop_front() else {
                break;
            };
            self.bytes = self.bytes.saturating_sub(old.size);
            self.seen.remove(&old.event.event_id);
            if self.connection.is_some()
                && stream_sequence(&old.event.sequence).unwrap_or(u64::MAX) > self.sent
            {
                self.break_stream();
                break;
            }
        }
        if self
            .snapshot
            .as_ref()
            .is_some_and(|s| now.duration_since(s.created) >= Duration::from_secs(SNAPSHOT_SECONDS))
        {
            self.snapshot = None;
        }
    }
}
pub(super) fn authorized<'a>(
    state: &'a mut State,
    identity: &Authenticated,
    connection: &str,
    id: &str,
) -> Result<&'a mut Stream, Error> {
    let stream = state.streams.get_mut(id).ok_or(Error::ResyncRequired)?;
    if stream.connection.as_deref() != Some(connection)
        || stream.identity.principal_id != identity.principal_id
        || stream.identity.session_id != identity.session_id
    {
        return Err(Error::ResyncRequired);
    }
    if !stream.matches(identity) {
        stream.break_stream();
    }
    if stream.broken {
        return Err(Error::ResyncRequired);
    }
    Ok(stream)
}
pub(super) fn gc(state: &mut State, now: Instant) {
    state.streams.retain(|_, s| {
        !s.detached_at
            .is_some_and(|at| now.duration_since(at) >= Duration::from_secs(STREAM_SECONDS))
    });
    for s in state.streams.values_mut() {
        s.trim(now);
    }
}
fn scope(identity: &Authenticated, params: &SubscribeParams) -> Result<(), Error> {
    if params
        .points_limit
        .is_some_and(|n| n > 1000 || !params.domains.contains(&StreamDomain::Points))
    {
        return Err(Error::Invalid("invalid points projection limit"));
    }
    if params.domains.is_empty() {
        return Err(Error::Invalid(
            "at least one subscription domain is required",
        ));
    }
    if params.feed
        || params.domains.iter().any(|d| {
            !matches!(
                d,
                StreamDomain::Workflows
                    | StreamDomain::Points
                    | StreamDomain::Automation
                    | StreamDomain::Commands
            )
        })
    {
        return Err(Error::Unsupported);
    }
    let caps = &identity.grant.capabilities;
    if params.domains.contains(&StreamDomain::Workflows)
        && !caps.contains(&Capability::WorkflowsRead)
        || params.domains.contains(&StreamDomain::Points)
            && (!caps.contains(&Capability::PointsRead)
                || identity.grant.resources.contains_key(&ResourceKind::Live))
        || params.domains.contains(&StreamDomain::Automation)
            && (!caps.contains(&Capability::AutomationRead)
                || identity.grant.resources.contains_key(&ResourceKind::Live))
        || params.domains.contains(&StreamDomain::Commands)
            && (!caps.contains(&Capability::CommandsRead)
                || !(caps.contains(&Capability::WorkflowsRead)
                    || (caps.contains(&Capability::PointsRead)
                        || caps.contains(&Capability::AutomationRead))
                        && !identity.grant.resources.contains_key(&ResourceKind::Live)))
    {
        return Err(Error::Forbidden);
    }
    Ok(())
}

pub(super) fn resource_allowed(grant: &crate::Grant, resource: &ResourceVersion) -> bool {
    grant.allows_scoped_resource(resource.kind, &resource.scope, &resource.id)
        && match resource.kind {
            ResourceKind::Workflow => grant.capabilities.contains(&Capability::WorkflowsRead),
            ResourceKind::Viewer => {
                grant.capabilities.contains(&Capability::PointsRead)
                    && !grant.resources.contains_key(&ResourceKind::Live)
            }
            ResourceKind::Automation => {
                grant.capabilities.contains(&Capability::AutomationRead)
                    && !grant.resources.contains_key(&ResourceKind::Live)
            }
            _ => false,
        }
}
fn room(
    state: &mut State,
    identity: &Authenticated,
    connection: &str,
    limits: &RemoteLimits,
    replacing: Option<&str>,
) -> Result<(), Error> {
    if state.streams.values().any(|s| {
        s.connection.as_deref() == Some(connection)
            && (s.identity.principal_id != identity.principal_id
                || s.identity.session_id != identity.session_id)
    }) {
        return Err(Error::Forbidden);
    }
    let others: Vec<_> = state
        .streams
        .iter()
        .filter(|(id, s)| {
            s.connection.is_some()
                && s.connection.as_deref() != Some(connection)
                && replacing != Some(id.as_str())
        })
        .collect();
    if others.len() >= limits.connections as usize
        || others
            .iter()
            .filter(|(_, s)| s.identity.principal_id == identity.principal_id)
            .count()
            >= limits.connections_per_principal as usize
    {
        return Err(Error::Overloaded);
    }
    Ok(())
}
impl StreamHub {
    pub fn subscribe(
        &self,
        identity: &Authenticated,
        connection: &str,
        instance: &str,
        params: SubscribeParams,
        limits: &RemoteLimits,
    ) -> Result<SubscribeResult, Error> {
        scope(identity, &params)?;
        limits.validate()?;
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        room(&mut state, identity, connection, limits, None)?;
        state
            .streams
            .retain(|_, s| s.connection.as_deref() != Some(connection));
        while state.streams.len() >= limits.connections as usize * 2 {
            let oldest = state
                .streams
                .iter()
                .filter_map(|(id, s)| s.detached_at.map(|at| (at, id.clone())))
                .min()
                .map(|(_, id)| id);
            if let Some(id) = oldest {
                state.streams.remove(&id);
            } else {
                return Err(Error::Overloaded);
            }
        }
        let id = Uuid::new_v4().to_string();
        let result = SubscribeResult {
            points_limit: params.points_limit,
            subscription_id: id.clone(),
            host_instance_id: instance.into(),
            grant_revision: identity.grant_revision.clone(),
            domains: params.domains.clone(),
            retained_events: STREAM_EVENTS,
            retained_bytes: STREAM_BYTES,
            retention_seconds: STREAM_SECONDS,
            snapshot_seconds: SNAPSHOT_SECONDS,
        };
        state.streams.insert(
            id,
            Stream {
                points_limit: params.points_limit,
                identity: identity.clone(),
                connection: Some(connection.into()),
                detached_at: None,
                host_instance: instance.into(),
                domains: params.domains,
                requests_per_second: limits.requests_per_second,
                request_burst: limits.request_burst,
                sequence: 0,
                sent: 0,
                events: VecDeque::new(),
                seen: BTreeSet::new(),
                bytes: 0,
                broken: false,
                baseline_complete: false,
                snapshot: None,
            },
        );
        Ok(result)
    }
    pub fn validate(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
    ) -> Result<BTreeSet<StreamDomain>, Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        Ok(authorized(&mut state, identity, connection, id)?
            .domains
            .clone())
    }
    pub fn points_limit(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
    ) -> Result<Option<u16>, Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        Ok(authorized(&mut state, identity, connection, id)?.points_limit)
    }
    pub fn detach(&self, connection: &str) {
        if let Ok(mut state) = self.state.lock() {
            let now = Instant::now();
            gc(&mut state, now);
            for s in state
                .streams
                .values_mut()
                .filter(|s| s.connection.as_deref() == Some(connection))
            {
                s.connection = None;
                s.detached_at = Some(now);
                s.snapshot = None;
                if !s.baseline_complete {
                    s.break_stream();
                }
            }
        }
    }
    pub fn prune(&self) {
        if let Ok(mut state) = self.state.lock() {
            gc(&mut state, Instant::now());
        }
    }
    pub fn resume(
        &self,
        identity: &Authenticated,
        connection: &str,
        instance: &str,
        params: ResumeParams,
        limits: &RemoteLimits,
    ) -> Result<ResumeResult, Error> {
        limits.validate()?;
        let sequence = stream_sequence(&params.last_applied_sequence)?;
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        let s = state
            .streams
            .get(&params.subscription_id)
            .ok_or(Error::ResyncRequired)?;
        if !s.matches(identity)
            || s.broken
            || !s.baseline_complete
            || s.host_instance != instance
            || params.host_instance_id != instance
            || s.connection.as_ref().is_some_and(|old| old != connection)
            || sequence > s.sequence
            || s.events.front().is_some_and(|first| {
                sequence.saturating_add(1)
                    < stream_sequence(&first.event.sequence).unwrap_or(u64::MAX)
            })
            || s.events.is_empty() && sequence < s.sequence
        {
            return Err(Error::ResyncRequired);
        }
        room(
            &mut state,
            identity,
            connection,
            limits,
            Some(&params.subscription_id),
        )?;
        // A resumed stream can follow a configuration change that lowered
        // connection limits. Bound retained state under the new budget too.
        while state.streams.len() > limits.connections as usize * 2 {
            let oldest = state
                .streams
                .iter()
                .filter(|(id, _)| *id != &params.subscription_id)
                .filter_map(|(id, s)| s.detached_at.map(|at| (at, id.clone())))
                .min()
                .map(|(_, id)| id);
            if let Some(id) = oldest {
                state.streams.remove(&id);
            } else {
                return Err(Error::Overloaded);
            }
        }
        state.streams.retain(|id, s| {
            id == &params.subscription_id || s.connection.as_deref() != Some(connection)
        });
        let s = state
            .streams
            .get_mut(&params.subscription_id)
            .ok_or(Error::ResyncRequired)?;
        s.connection = Some(connection.into());
        s.detached_at = None;
        s.identity = identity.clone();
        s.requests_per_second = limits.requests_per_second;
        s.request_burst = limits.request_burst;
        s.sent = sequence;
        Ok(ResumeResult {
            subscription_id: params.subscription_id,
            host_instance_id: instance.into(),
            grant_revision: identity.grant_revision.clone(),
            after_sequence: sequence.to_string(),
            through_sequence: s.sequence.to_string(),
            replaying: sequence < s.sequence,
        })
    }
    pub fn publish(&self, event: &CoordinationEvent) -> Result<(), Error> {
        let now = Instant::now();
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, now);
        for (id, s) in state.streams.iter_mut().filter(|(_, s)| !s.broken) {
            let grant = &s.identity.grant;
            let (event_id, resource, actor, command_id, topic, data) = match event {
                CoordinationEvent::Invalidated {
                    event_id,
                    resource,
                    principal_id,
                    command_id,
                    change,
                    ..
                } if resource_allowed(grant, resource)
                    && match resource.kind {
                        ResourceKind::Workflow => s.domains.contains(&StreamDomain::Workflows),
                        ResourceKind::Viewer => s.domains.contains(&StreamDomain::Points),
                        ResourceKind::Automation => s.domains.contains(&StreamDomain::Automation),
                        _ => false,
                    } =>
                {
                    (
                        event_id,
                        resource,
                        principal_id,
                        if *principal_id == s.identity.principal_id {
                            command_id.clone()
                        } else {
                            None
                        },
                        if resource.kind == ResourceKind::Viewer {
                            "points.changed"
                        } else if resource.kind == ResourceKind::Automation {
                            "automation.changed"
                        } else {
                            "workflow.changed"
                        },
                        if resource.kind == ResourceKind::Automation {
                            serde_json::json!({"id":resource.id,"kind":resource.scope,"change":change})
                        } else {
                            serde_json::json!({"id":resource.id,"change":change})
                        },
                    )
                }
                CoordinationEvent::CommandChanged {
                    event_id,
                    resource,
                    principal_id,
                    command,
                    ..
                } if s.domains.contains(&StreamDomain::Commands)
                    && *principal_id == s.identity.principal_id
                    && grant.capabilities.contains(&Capability::CommandsRead)
                    && resource_allowed(grant, resource) =>
                {
                    (
                        event_id,
                        resource,
                        principal_id,
                        Some(command.command_id.clone()),
                        "command.changed",
                        serde_json::json!({"command":command}),
                    )
                }
                _ => continue,
            };
            if s.seen.contains(event_id) {
                continue;
            }
            let Some(sequence) = s.sequence.checked_add(1).filter(|n| *n <= i64::MAX as u64) else {
                s.break_stream();
                continue;
            };
            let event = ReliableEvent {
                topic: topic.into(),
                data,
                host_instance_id: s.host_instance.clone(),
                subscription_id: id.clone(),
                sequence: sequence.to_string(),
                event_id: event_id.clone(),
                resource: resource.clone(),
                actor: StreamActor {
                    principal_id: actor.clone(),
                },
                command_id,
            };
            let size = serde_json::to_vec(&event)?.len();
            s.sequence = sequence;
            s.bytes = s.bytes.saturating_add(size);
            s.seen.insert(event.event_id.clone());
            s.events.push_back(Retained {
                event,
                size,
                at: now,
            });
            s.trim(now);
        }
        Ok(())
    }
    pub fn next(
        &self,
        identity: &Authenticated,
        connection: &str,
    ) -> Result<Option<ReliableEvent>, Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        let id = state
            .streams
            .iter()
            .find(|(_, s)| s.connection.as_deref() == Some(connection))
            .map(|(id, _)| id.clone());
        let Some(id) = id else {
            return Ok(None);
        };
        let s = authorized(&mut state, identity, connection, &id)?;
        if !s.baseline_complete {
            return Ok(None);
        }
        Ok(s.events
            .iter()
            .find(|e| stream_sequence(&e.event.sequence).unwrap_or(0) > s.sent)
            .map(|e| e.event.clone()))
    }
    pub fn sent(
        &self,
        identity: &Authenticated,
        connection: &str,
        event: &ReliableEvent,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        let s = authorized(&mut state, identity, connection, &event.subscription_id)?;
        let sequence = stream_sequence(&event.sequence)?;
        if sequence != s.sent.saturating_add(1) || sequence > s.sequence {
            return Err(Error::ResyncRequired);
        }
        s.sent = sequence;
        Ok(())
    }
}
