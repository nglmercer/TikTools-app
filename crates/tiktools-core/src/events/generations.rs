//! Trusted native generation bindings and scoped ingress-loss evidence.
use super::{valid_creator_id, EventBus, FeedHistoryState};

impl FeedHistoryState {
    pub(super) fn rejects_generation(&self, generation: u64) -> bool {
        self.unverified_generations.contains(&generation)
            || self
                .unverified_expired_through
                .is_some_and(|old| generation <= old)
    }

    pub(super) fn reject_generation(&mut self, generation: u64) {
        self.live_generations.remove(&generation);
        self.pending_ingress_gaps.remove(&generation);
        self.unverified_generations.insert(generation);
        if self.unverified_generations.len() > 128 {
            if let Some(evicted) = self.unverified_generations.pop_first() {
                self.unverified_expired_through = Some(
                    self.unverified_expired_through
                        .map_or(evicted, |old| old.max(evicted)),
                );
            }
        }
    }
}

impl EventBus {
    /// Bind a native transport generation to a creator obtained from its
    /// stamped Connected event after its room identity has been verified.
    /// Only the ingress pipeline may call this; payload fields are untrusted.
    pub(crate) fn bind_live_generation(&self, generation: u64, creator: &str) {
        if !valid_creator_id(creator) {
            return;
        }
        // Match the publication lock order: sequence, then history.
        let sequence = *self.ui_sequence.lock().unwrap_or_else(|e| e.into_inner());
        let mut history = self.feed_history.lock().unwrap_or_else(|e| e.into_inner());
        if history.rejects_generation(generation) {
            return;
        }
        if history
            .live_generations
            .get(&generation)
            .is_some_and(|id| id != creator)
        {
            // An ambiguous generation must never send another creator's
            // loss indication to this caller.
            history.scoped_unavailable = true;
            history.reject_generation(generation);
            return;
        }
        history
            .live_generations
            .insert(generation, creator.to_owned());
        while history.live_generations.len() > 128 {
            history.live_generations.pop_first();
        }
        if history.pending_ingress_gaps.remove(&generation) {
            if let Some(stream) =
                history.creator_stream(creator, sequence, |bytes| getrandom::fill(bytes).is_ok())
            {
                stream.ingress_gap = true;
                stream.last_global = sequence;
            }
        }
    }

    /// Mark a native generation as lacking a verified room/creator binding.
    /// A later event must not turn its caller-supplied label into authority.
    pub(crate) fn reject_unverified_live_generation(&self, generation: u64) {
        let mut history = self.feed_history.lock().unwrap_or_else(|e| e.into_inner());
        if history.live_generations.contains_key(&generation) {
            // A conflicting lifecycle for one generation makes previous
            // attribution ambiguous; do not serve a potentially mixed feed.
            history.scoped_unavailable = true;
        }
        history.reject_generation(generation);
    }

    /// Only native Connected evidence may authorize a creator-scoped path.
    pub(crate) fn is_verified_live_generation(&self, generation: u64, creator: &str) -> bool {
        let history = self.feed_history.lock().unwrap_or_else(|e| e.into_inner());
        !history.rejects_generation(generation)
            && history
                .live_generations
                .get(&generation)
                .is_some_and(|id| id == creator)
    }

    /// Record loss for its stamped native generation without exposing counts.
    /// Unknown generations stay pending until a trusted identity is observed.
    pub(crate) fn record_live_ingress_gap(&self, generation: u64) {
        let sequence = *self.ui_sequence.lock().unwrap_or_else(|e| e.into_inner());
        let mut history = self.feed_history.lock().unwrap_or_else(|e| e.into_inner());
        if history.scoped_unavailable || history.rejects_generation(generation) {
            return;
        }
        if let Some(creator) = history.live_generations.get(&generation).cloned() {
            if let Some(stream) =
                history.creator_stream(&creator, sequence, |bytes| getrandom::fill(bytes).is_ok())
            {
                stream.ingress_gap = true;
                stream.last_global = sequence;
            }
        } else if !history.pending_ingress_gaps.contains(&generation) {
            // A bounded failure is safer than silently attributing or
            // forgetting losses from unknown generations.
            if history.pending_ingress_gaps.len() >= 128 {
                history.scoped_unavailable = true;
            } else {
                history.pending_ingress_gaps.insert(generation);
            }
        }
    }
}
