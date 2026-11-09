use super::{
    hub::{authorized, gc, resource_allowed},
    *,
};
use crate::{Authenticated, Error};
use std::time::Instant;
use uuid::Uuid;

pub(super) struct Snapshot {
    pub created: Instant,
    pages: Vec<SnapshotPage>,
    tokens: Vec<String>,
}
impl StreamHub {
    pub fn begin_snapshot(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        let s = authorized(&mut state, identity, connection, id)?;
        s.baseline_complete = false;
        s.snapshot = None;
        Ok(())
    }
    pub fn validate_snapshot(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
        snapshot_id: &str,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        let s = authorized(&mut state, identity, connection, id)?;
        if !s
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.pages.first())
            .is_some_and(|page| page.snapshot_id == snapshot_id)
        {
            return Err(Error::ResyncRequired);
        }
        Ok(())
    }
    /// Caller holds the same state barrier as writers and publish. Records
    /// were already permission filtered and bounded by the owning database.
    pub fn capture(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
        records: Vec<(Value, ResourceVersion)>,
        page_bytes: usize,
    ) -> Result<SnapshotPage, Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        let now = Instant::now();
        gc(&mut state, now);
        let s = authorized(&mut state, identity, connection, id)?;
        if !(1024..=512 * 1024).contains(&page_bytes) {
            return Err(Error::Invalid("invalid snapshot page budget"));
        }
        s.baseline_complete = false;
        s.snapshot = None;
        let snapshot_id = Uuid::new_v4().to_string();
        let empty = SnapshotPage {
            points_limit: s.points_limit,
            subscription_id: id.into(),
            snapshot_id,
            host_instance_id: s.host_instance.clone(),
            grant_revision: identity.grant_revision.clone(),
            through_sequence: s.sequence.to_string(),
            domains: s.domains.clone(),
            workflows: Vec::new(),
            points: Vec::new(),
            automation: Vec::new(),
            resource_versions: Vec::new(),
            next_page_token: None,
        };
        let mut pages = vec![empty.clone()];
        let mut used = 0usize;
        // Include page envelopes, next tokens and the retained token index in
        // the snapshot budget, rather than counting only workflow bodies.
        // Empty points arrays are omitted on the wire. Reserve their field
        // envelope for every page that may later contain a viewer.
        let page_overhead = serde_json::to_vec(&empty)?.len()
            + 38
            + 36
            + if s.domains.contains(&StreamDomain::Points) {
                12
            } else {
                0
            }
            + if s.domains.contains(&StreamDomain::Automation) {
                16
            } else {
                0
            };
        let mut total = page_overhead;
        if s.points_limit.is_some_and(|n| {
            records
                .iter()
                .filter(|(_, v)| v.kind == crate::ResourceKind::Viewer)
                .count()
                > usize::from(n)
        }) {
            return Err(Error::TooLarge);
        }
        for (record, version) in records {
            let (domain, field) = match version.kind {
                crate::ResourceKind::Workflow => (StreamDomain::Workflows, "id"),
                crate::ResourceKind::Viewer => (StreamDomain::Points, "uniqueId"),
                crate::ResourceKind::Automation => (StreamDomain::Automation, "id"),
                _ => return Err(Error::Forbidden),
            };
            if !s.domains.contains(&domain)
                || !resource_allowed(&identity.grant, &version)
                || record.get(field).and_then(Value::as_str) != Some(version.id.as_str())
            {
                return Err(Error::Forbidden);
            }
            if version.kind == crate::ResourceKind::Automation {
                let summary: crate::AutomationSummary =
                    serde_json::from_value(record.clone()).map_err(|_| Error::Forbidden)?;
                if summary.kind.scope() != version.scope
                    || !summary.definition_redacted
                    || summary.name.chars().count() > 160
                {
                    return Err(Error::Forbidden);
                }
            }
            let bytes =
                serde_json::to_vec(&record)?.len() + serde_json::to_vec(&version)?.len() + 2;
            total = total.checked_add(bytes).ok_or(Error::TooLarge)?;
            if bytes > page_bytes || total > SNAPSHOT_BYTES {
                return Err(Error::TooLarge);
            }
            if used + bytes > page_bytes {
                total = total.checked_add(page_overhead).ok_or(Error::TooLarge)?;
                if total > SNAPSHOT_BYTES {
                    return Err(Error::TooLarge);
                }
                pages.push(empty.clone());
                used = 0;
            }
            used += bytes;
            let page = pages.last_mut().ok_or(Error::ResyncRequired)?;
            match domain {
                StreamDomain::Workflows => page.workflows.push(record),
                StreamDomain::Points => page.points.push(record),
                StreamDomain::Automation => page.automation.push(record),
                _ => unreachable!(),
            }
            page.resource_versions.push(version);
        }
        // Even under ideal conditions, a full bucket at capture followed by
        // an uninterrupted refill cannot admit more requests than this. The
        // first page is returned by the capture call and costs no additional
        // request. Other traffic, network latency and bucket state can still
        // make a smaller snapshot expire; this is an impossibility check, not
        // a promise that the budget will be available to this connection.
        let optimistic_requests =
            u64::from(s.request_burst) + u64::from(s.requests_per_second) * SNAPSHOT_SECONDS;
        if pages.len().saturating_sub(1) as u64 > optimistic_requests {
            return Err(Error::SnapshotRateBudget);
        }
        let tokens: Vec<_> = (0..pages.len())
            .map(|_| Uuid::new_v4().to_string())
            .collect();
        for (index, page) in pages.iter_mut().enumerate() {
            page.next_page_token = tokens.get(index + 1).cloned();
        }
        let first = pages[0].clone();
        s.sent = s.sequence;
        s.baseline_complete = pages.len() == 1;
        s.snapshot = Some(Snapshot {
            created: now,
            pages,
            tokens,
        });
        Ok(first)
    }
    pub fn page(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
        token: &str,
    ) -> Result<SnapshotPage, Error> {
        let mut state = self.state.lock().map_err(|_| Error::ResyncRequired)?;
        gc(&mut state, Instant::now());
        let s = authorized(&mut state, identity, connection, id)?;
        let snapshot = s.snapshot.as_ref().ok_or(Error::ResyncRequired)?;
        let index = snapshot
            .tokens
            .iter()
            .position(|v| v == token)
            .ok_or(Error::ResyncRequired)?;
        let page = snapshot.pages[index].clone();
        if page.next_page_token.is_none() {
            s.baseline_complete = true;
        }
        Ok(page)
    }
}
