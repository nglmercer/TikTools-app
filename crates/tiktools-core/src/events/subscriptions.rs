//! Lane-aware subscription ownership, priority and lag/close handling.
use super::{
    DomainEvent, DomainRecvError, DomainSubscription, DomainTryRecvError, EventBus,
    PluginDomainSubscription, PublishedEvent,
};
use std::sync::Arc;
use tokio::sync::broadcast;

impl EventBus {
    /// Whether a presentation subscriber can receive lossy snapshots.
    pub fn has_domain_subscribers(&self) -> bool {
        self.lossy.receiver_count() > 0 || self.plugin_lossy.receiver_count() > 0
    }

    pub fn subscribe_domain(&self) -> DomainSubscription {
        self.subscribe(true)
    }

    /// Completion/state projections do not request the compatibility full
    /// automation history on every action. Default domain subscribers still
    /// request the complete historical contract.
    pub fn subscribe_projection(&self) -> DomainSubscription {
        self.subscribe(false)
    }

    /// Only executable domain observers use this private subscription. Separate
    /// rings keep presentation-only traffic outside both plugin calls and the
    /// executable reliable-gap recovery path.
    pub(crate) fn subscribe_plugin_domain(&self) -> PluginDomainSubscription {
        let sequence = self.ui_sequence.lock().unwrap_or_else(|e| e.into_inner());
        PluginDomainSubscription {
            inner: DomainSubscription {
                history_owner: None,
                reliable: self.plugin_reliable.subscribe(),
                lossy: self.plugin_lossy.subscribe(),
                last_ui_sequence: *sequence,
            },
        }
    }

    pub fn history_requested(&self) -> bool {
        self.history_subscribers
            .load(std::sync::atomic::Ordering::Acquire)
            > 0
    }

    pub(super) fn subscribe(&self, history: bool) -> DomainSubscription {
        let sequence = self.ui_sequence.lock().unwrap_or_else(|e| e.into_inner());
        if history {
            self.history_subscribers
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        }
        DomainSubscription {
            history_owner: history.then(|| self.history_subscribers.clone()),
            reliable: self.reliable.subscribe(),
            lossy: self.lossy.subscribe(),
            last_ui_sequence: *sequence,
        }
    }
}

impl PluginDomainSubscription {
    pub(crate) async fn recv_shared(&mut self) -> Result<Arc<DomainEvent>, DomainRecvError> {
        loop {
            let published = self.inner.recv_published().await?;
            if published.observer_allowed {
                return Ok(published.event);
            }
        }
    }
}

impl Drop for DomainSubscription {
    fn drop(&mut self) {
        if let Some(owner) = &self.history_owner {
            owner.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        }
    }
}

impl DomainSubscription {
    pub fn ui_sequence(&self) -> u64 {
        self.last_ui_sequence
    }

    /// Shared receive lets projections discard unused events without cloning
    /// their entire payload (notably the 60-item automation history).
    pub async fn recv_shared(&mut self) -> Result<Arc<DomainEvent>, DomainRecvError> {
        let published = self.recv_published().await?;
        if let Some(sequence) = published.ui_sequence {
            self.last_ui_sequence = sequence;
        }
        Ok(published.event)
    }

    pub async fn recv(&mut self) -> Result<DomainEvent, DomainRecvError> {
        self.recv_shared().await.map(|event| (*event).clone())
    }

    pub fn try_recv(&mut self) -> Result<DomainEvent, DomainTryRecvError> {
        let published = self.try_recv_published()?;
        if let Some(sequence) = published.ui_sequence {
            self.last_ui_sequence = sequence;
        }
        Ok((*published.event).clone())
    }

    /// Receives the next event, reliable lane first. A lagged lane
    /// reports once and then streams its retained backlog; the other
    /// lane is unaffected.
    pub(super) async fn recv_published(&mut self) -> Result<PublishedEvent, DomainRecvError> {
        match self.try_recv_published() {
            Err(DomainTryRecvError::Empty) => {}
            Ok(event) => return Ok(event),
            Err(DomainTryRecvError::ReliableLagged(skipped)) => {
                return Err(DomainRecvError::ReliableLagged(skipped));
            }
            Err(DomainTryRecvError::LossyLagged(skipped)) => {
                return Err(DomainRecvError::LossyLagged(skipped));
            }
            Err(DomainTryRecvError::Closed) => return Err(DomainRecvError::Closed),
        }
        // Both lanes are empty: wait biased toward reliable. A lossy
        // wake may overtake a reliable message published a moment later;
        // the next `recv` takes reliable first, so control events stay
        // prompt while feed stays merely ordered.
        tokio::select! {
            biased;
            message = self.reliable.recv() => {
                match message {
                    // The bus died mid-wait: drain lossy stragglers buffered
                    // before the drop instead of losing them to Closed.
                    Err(broadcast::error::RecvError::Closed) => {
                        match self.lossy.try_recv() {
                            Ok(straggler) => Ok(straggler),
                            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                                Err(DomainRecvError::LossyLagged(skipped))
                            }
                            Err(_) => Err(DomainRecvError::Closed),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        Err(DomainRecvError::ReliableLagged(skipped))
                    }
                    Ok(event) => Ok(event),
                }
            }
            message = self.lossy.recv() => {
                match message {
                    // The reliable lane may still hold buffered messages
                    // (or stay open), so never strand them behind a lossy
                    // close: fall back to the reliable backlog first.
                    Err(broadcast::error::RecvError::Closed) => match self.reliable.try_recv() {
                        Ok(event) => Ok(event),
                        Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                            Err(DomainRecvError::ReliableLagged(skipped))
                        }
                        Err(_) => Err(DomainRecvError::Closed),
                    },
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        Err(DomainRecvError::LossyLagged(skipped))
                    }
                    Ok(event) => Ok(event),
                }
            }
        }
    }

    pub(super) fn try_recv_published(&mut self) -> Result<PublishedEvent, DomainTryRecvError> {
        use broadcast::error::TryRecvError;
        match self.reliable.try_recv() {
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Lagged(skipped)) => {
                return Err(DomainTryRecvError::ReliableLagged(skipped));
            }
            Err(TryRecvError::Closed) => {
                // The bus is gone: drain lossy stragglers before Closed.
                return match self.lossy.try_recv() {
                    Ok(event) => Ok(event),
                    Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => {
                        Err(DomainTryRecvError::Closed)
                    }
                    Err(TryRecvError::Lagged(skipped)) => {
                        Err(DomainTryRecvError::LossyLagged(skipped))
                    }
                };
            }
            Ok(event) => return Ok(event),
        }
        match self.lossy.try_recv() {
            // Both lanes empty and the reliable sender is alive (it would
            // report Closed, not Empty, otherwise), so more events can
            // still arrive: report Empty, never Closed. Both senders live
            // in one `EventBus` and drop together, so a lossy Closed here
            // is unreachable; map it to Empty for the same reason.
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => Err(DomainTryRecvError::Empty),
            Err(TryRecvError::Lagged(skipped)) => Err(DomainTryRecvError::LossyLagged(skipped)),
            Ok(event) => Ok(event),
        }
    }
}
