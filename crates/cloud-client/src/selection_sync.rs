//! Selection change pages, compare-and-set pushes and atomic state rebuilding.
use super::*;

impl Session {
    /// Apply one change page atomically in memory; persist it only after this returns.
    pub async fn pull_page(&self, state: &mut SyncState) -> Result<bool, SyncError> {
        self.pull_page_with_count(state)
            .await
            .map(|(has_more, _)| has_more)
    }

    /// Apply one page and report both the continuation flag and the number
    /// of change records received, including records in the final page.
    pub async fn pull_page_with_count(
        &self,
        state: &mut SyncState,
    ) -> Result<(bool, usize), SyncError> {
        self.verify_account(state)?;
        let page: Page = self
            .get(
                "api/v1/native/sync/changes",
                &[("cursor", state.cursor.to_string())],
            )
            .await?;
        if page.cursor < state.cursor || (page.has_more && page.cursor == state.cursor) {
            return Err(SyncError::Protocol);
        }
        let mut candidate = state.clone();
        let mut last = state.cursor;
        let count = page.changes.len();
        for record in page.changes {
            if record.revision <= last || record.revision > page.cursor {
                return Err(SyncError::Protocol);
            }
            last = record.revision;
            candidate.apply(record)?;
        }
        if last != page.cursor {
            return Err(SyncError::Protocol);
        }
        candidate.cursor = page.cursor;
        *state = candidate;
        Ok((page.has_more, count))
    }

    /// Send one pending edit. A 409 leaves the edit intact for explicit resolution.
    pub async fn push(&self, state: &mut SyncState, key: &str) -> Result<(), SyncError> {
        self.verify_account(state)?;
        if !valid_key(key) {
            return Err(SyncError::Configuration);
        }
        let entry = state.entries.get(key).ok_or(SyncError::NoPendingEdit)?;
        let pending = entry.pending.as_ref().ok_or(SyncError::NoPendingEdit)?;
        if pending.conflict {
            return Err(SyncError::Conflict);
        }
        if pending.base_revision.is_none() && matches!(pending.edit, Edit::Delete) {
            // A previous write could have succeeded despite a lost response, or
            // another device could have created this record while we were offline.
            state
                .entries
                .get_mut(key)
                .unwrap()
                .pending
                .as_mut()
                .unwrap()
                .conflict = true;
            return Err(SyncError::Conflict);
        }
        let path = format!("api/v1/native/sync/collections/{COLLECTION}/{key}");
        let mut request = match &pending.edit {
            Edit::Put(value) => self.request(reqwest::Method::PUT, &path)?.json(value),
            Edit::Delete => self.request(reqwest::Method::DELETE, &path)?,
        };
        request = if let Some(revision) = pending.base_revision {
            request.header(IF_MATCH, format!("\"{revision}\""))
        } else {
            request.header(IF_NONE_MATCH, "*")
        };
        let response = request.send().await.map_err(|_| SyncError::Network)?;
        if response.status() == StatusCode::CONFLICT {
            state
                .entries
                .get_mut(key)
                .unwrap()
                .pending
                .as_mut()
                .unwrap()
                .conflict = true;
            return Err(SyncError::Conflict);
        }
        let record: Record = decode(response).await?;
        validate_record(&record)?;
        if (match &pending.edit {
            Edit::Put(value) => record.value.as_ref() != Some(value) || record.deleted,
            Edit::Delete => !record.deleted || record.value.is_some(),
        }) || record.key != key
            || pending
                .base_revision
                .is_some_and(|revision| record.revision <= revision)
        {
            return Err(SyncError::Protocol);
        }
        let entry = state.entries.get_mut(key).unwrap();
        entry.remote = Some(record);
        entry.pending = None;
        Ok(())
    }

    /// Rebuild after retention expiry without dropping pending edits. Snapshot and
    /// follow-up changes are committed to state only after every page succeeds.
    pub async fn rebuild(&self, state: &mut SyncState) -> Result<(), SyncError> {
        self.rebuild_with_count(state).await.map(|_| ())
    }

    /// Rebuild and report the number of snapshot and follow-up feed records
    /// consumed. State remains unchanged if any page fails validation.
    pub async fn rebuild_with_count(&self, state: &mut SyncState) -> Result<usize, SyncError> {
        self.verify_account(state)?;
        let head: Head = self.get("api/v1/native/sync/head", &[]).await?;
        if head.cursor < 0 {
            return Err(SyncError::Protocol);
        }
        let mut candidate = SyncState::new(state.origin.clone(), state.account_id.clone());
        candidate.cursor = head.cursor;
        let mut after: Option<String> = None;
        let mut complete = false;
        let mut count = 0usize;
        for _ in 0..MAX_PAGES {
            let params = after
                .as_ref()
                .map(|s| vec![("after", s.clone())])
                .unwrap_or_default();
            let listing: Listing = self
                .get("api/v1/native/sync/collections/plugin-selections", &params)
                .await?;
            let previous = after.clone();
            count = count
                .checked_add(listing.records.len())
                .ok_or(SyncError::Protocol)?;
            for record in listing.records {
                if after.as_ref().is_some_and(|a| record.key <= *a) {
                    return Err(SyncError::Protocol);
                }
                after = Some(record.key.clone());
                candidate.apply(record)?;
            }
            if let Some(next) = listing.next_after {
                if Some(&next) != after.as_ref() || previous.as_ref().is_some_and(|p| next <= *p) {
                    return Err(SyncError::Protocol);
                }
            } else {
                complete = true;
                break;
            }
        }
        if !complete {
            return Err(SyncError::TooManyPages);
        }
        for (key, old) in &state.entries {
            if let Some(pending) = &old.pending {
                let entry = candidate.entries.entry(key.clone()).or_default();
                let mut pending = pending.clone();
                if pending.base_revision != entry.remote.as_ref().map(|r| r.revision) {
                    pending.conflict = true;
                }
                entry.pending = Some(pending);
            }
        }
        for _ in 0..MAX_PAGES {
            let (more, received) = self.pull_page_with_count(&mut candidate).await?;
            count = count.checked_add(received).ok_or(SyncError::Protocol)?;
            if !more {
                *state = candidate;
                return Ok(count);
            }
        }
        Err(SyncError::TooManyPages)
    }
}
