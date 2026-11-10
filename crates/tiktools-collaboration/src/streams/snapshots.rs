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
    pub(super) tokens: Vec<String>,
    next_page: usize,
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
        self.capture_inner(identity, connection, id, records, page_bytes, None)
    }

    /// Pack each page for the negotiated frame, reserving the RPC envelope
    /// and the exact snapshot metadata. Final serialized pages are checked
    /// before any part of the baseline becomes visible.
    pub fn capture_with_frame(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
        records: Vec<(Value, ResourceVersion)>,
        frame_bytes: usize,
    ) -> Result<SnapshotPage, Error> {
        if !(4096..=1024 * 1024).contains(&frame_bytes) {
            return Err(Error::Invalid("invalid snapshot frame budget"));
        }
        self.capture_inner(
            identity,
            connection,
            id,
            records,
            snapshot_page_bytes(frame_bytes),
            Some(frame_bytes),
        )
    }

    fn capture_inner(
        &self,
        identity: &Authenticated,
        connection: &str,
        id: &str,
        records: Vec<(Value, ResourceVersion)>,
        page_bytes: usize,
        frame_bytes: Option<usize>,
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
            competitions: Vec::new(),
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
            }
            + if s.domains.contains(&StreamDomain::Competitions) {
                18
            } else {
                0
            };
        let data_budget = if frame_bytes.is_some() {
            page_bytes
                .checked_sub(page_overhead)
                .ok_or(Error::TooLarge)?
        } else {
            page_bytes
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
                crate::ResourceKind::Competition => (StreamDomain::Competitions, "id"),
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
            if bytes > data_budget || total > SNAPSHOT_BYTES {
                return Err(Error::TooLarge);
            }
            if used + bytes > data_budget {
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
                StreamDomain::Competitions => page.competitions.push(record),
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
        // Expiry is inclusive: a request admitted exactly at 30 seconds
        // cannot read the snapshot. Exclude that last refill from the upper
        // bound even when the bucket was full when capture completed.
        let optimistic_requests = u64::from(s.request_burst)
            + (u64::from(s.requests_per_second) * SNAPSHOT_SECONDS).saturating_sub(1);
        if pages.len().saturating_sub(1) as u64 > optimistic_requests {
            return Err(Error::SnapshotRateBudget);
        }
        let tokens: Vec<_> = (0..pages.len())
            .map(|_| Uuid::new_v4().to_string())
            .collect();
        for (index, page) in pages.iter_mut().enumerate() {
            page.next_page_token = tokens.get(index + 1).cloned();
        }
        if let Some(frame_bytes) = frame_bytes {
            for page in &pages {
                if serde_json::to_vec(page)?
                    .len()
                    .checked_add(SNAPSHOT_RPC_RESERVE)
                    .is_none_or(|n| n > frame_bytes)
                {
                    return Err(Error::TooLarge);
                }
            }
        }
        let first = pages[0].clone();
        s.sent = s.sequence;
        s.baseline_complete = pages.len() == 1;
        s.snapshot = Some(Snapshot {
            created: now,
            pages,
            tokens,
            next_page: 1,
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
        let snapshot = s.snapshot.as_mut().ok_or(Error::ResyncRequired)?;
        let index = snapshot
            .tokens
            .iter()
            .position(|v| v == token)
            .ok_or(Error::ResyncRequired)?;
        // A replay of an already returned page is harmless, but a caller
        // must not finish a baseline by skipping an undispatched page.
        if index == 0 || index > snapshot.next_page {
            return Err(Error::ResyncRequired);
        }
        let page = snapshot.pages[index].clone();
        if index == snapshot.next_page {
            snapshot.next_page += 1;
        }
        if snapshot.next_page == snapshot.pages.len() && page.next_page_token.is_none() {
            s.baseline_complete = true;
        }
        Ok(page)
    }
}
