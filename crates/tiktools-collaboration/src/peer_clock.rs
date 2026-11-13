//! Offline grants use suspend-aware boot time, never adjustable calendar time.
//! A process restart preserves the deadline; another boot needs fresh Cloud
//! validation because elapsed time across a power-off cannot be trusted locally.
mod system;
use crate::{Error, PEER_OFFLINE_TTL_MS};

#[derive(Clone, Debug)]
pub(crate) struct Stamp {
    pub(crate) boot: String,
    pub(crate) elapsed_ms: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct PeerLease {
    pub(crate) boot: String,
    pub(crate) issued_ms: i64,
    pub(crate) until_ms: i64,
}
impl PeerLease {
    pub(crate) fn issue() -> Result<Self, Error> {
        let at = sample().ok_or(Error::Unauthorized)?;
        Ok(Self {
            boot: at.boot,
            issued_ms: at.elapsed_ms,
            until_ms: at
                .elapsed_ms
                .checked_add(PEER_OFFLINE_TTL_MS)
                .ok_or(Error::Unauthorized)?,
        })
    }

    pub(crate) fn remaining(&self) -> Result<i64, Error> {
        let at = sample().ok_or(Error::Unauthorized)?;
        if self.issued_ms < 0
            || self.until_ms.checked_sub(self.issued_ms) != Some(PEER_OFFLINE_TTL_MS)
            || at.boot != self.boot
            || at.elapsed_ms < self.issued_ms
            || at.elapsed_ms >= self.until_ms
        {
            return Err(Error::Unauthorized);
        }
        Ok(self.until_ms - at.elapsed_ms)
    }
}

#[cfg(test)]
std::thread_local! {
    pub(crate) static TEST_CLOCK: std::cell::RefCell<Option<Stamp>> = const { std::cell::RefCell::new(None) };
}

fn sample() -> Option<Stamp> {
    #[cfg(test)]
    if let Some(at) = TEST_CLOCK.with(|clock| clock.borrow().clone()) {
        return Some(at);
    }
    system::sample()
}
