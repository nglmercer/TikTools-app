use super::*;

impl WorkspaceState {
    pub fn entry(&self, key: &WorkspaceKey) -> Option<&WorkspaceEntry> {
        self.index(key).ok().map(|i| &self.entries[i])
    }

    pub fn local_generation(&self, key: &WorkspaceKey) -> Option<u64> {
        self.entry(key).and_then(|entry| {
            entry
                .next
                .as_ref()
                .map(|v| v.local_generation)
                .or_else(|| entry.pending.as_ref().map(|v| v.local_generation))
        })
    }

    pub fn ready_remote(&self) -> Vec<WorkspaceRecord> {
        if self.needs_rebuild {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|entry| entry.pending.is_none())
            .filter_map(|entry| entry.remote.clone())
            .collect()
    }

    pub fn pending_keys(&self) -> Vec<WorkspaceKey> {
        self.entries
            .iter()
            .filter(|entry| entry.pending.is_some())
            .map(|entry| entry.target.clone())
            .collect()
    }

    pub fn conflict_keys(&self) -> Vec<WorkspaceKey> {
        self.entries
            .iter()
            .filter(|entry| {
                entry
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.status != WorkspacePendingStatus::Queued)
            })
            .map(|entry| entry.target.clone())
            .collect()
    }
}
