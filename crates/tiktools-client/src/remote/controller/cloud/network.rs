//! Selection cancellation fences even work queued behind the lifecycle lock.
use super::*;
use tokio_util::sync::CancellationToken;
#[derive(Default)]
pub(crate) struct NetworkState {
    generation: u64,
    pending: Option<CancellationToken>,
}
pub(super) fn generation(slot: &std::sync::Mutex<NetworkState>) -> Result<u64, ClientError> {
    Ok(slot
        .lock()
        .map_err(|_| fail("internal", "Cloud connection lock unavailable"))?
        .generation)
}
pub(crate) fn cancel(slot: &std::sync::Mutex<NetworkState>) {
    if let Ok(mut state) = slot.lock() {
        state.generation = state.generation.wrapping_add(1);
        if let Some(pending) = state.pending.take() {
            pending.cancel();
        }
    }
}
pub(super) struct Pending<'a> {
    slot: &'a std::sync::Mutex<NetworkState>,
    generation: u64,
    pub token: CancellationToken,
}
impl Drop for Pending<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.slot.lock() {
            if state.generation == self.generation {
                state.pending = None;
            }
        }
    }
}
pub(super) fn begin(
    slot: &std::sync::Mutex<NetworkState>,
    generation: u64,
) -> Result<Pending<'_>, ClientError> {
    let mut state = slot
        .lock()
        .map_err(|_| fail("internal", "Cloud connection lock unavailable"))?;
    if state.generation != generation {
        return Err(fail("cancelled", "Cloud connection selection changed"));
    }
    let token = CancellationToken::new();
    state.pending = Some(token.clone());
    Ok(Pending {
        slot,
        generation,
        token,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_fences_queued_work_and_an_old_guard_cannot_clear_a_new_operation() {
        let slot = std::sync::Mutex::new(NetworkState::default());
        let queued = generation(&slot).unwrap();
        cancel(&slot);
        assert!(begin(&slot, queued).is_err());
        let old = begin(&slot, generation(&slot).unwrap()).unwrap();
        cancel(&slot);
        assert!(old.token.is_cancelled());
        let fresh = begin(&slot, generation(&slot).unwrap()).unwrap();
        drop(old);
        cancel(&slot);
        assert!(fresh.token.is_cancelled());
    }
}
