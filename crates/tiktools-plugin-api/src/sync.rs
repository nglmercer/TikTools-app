//! Poison-recovery helpers for explicitly recoverable host-side locks.
//!
//! Production locks guard runtime state shared across threads (plugin
//! registry, connection context, diagnostics). A panic while holding a lock
//! poisons it; historical code turned that into a host crash via
//! `expect(...)`.
//!
//! Policy, by lock:
//!
//! - Recoverable runtime state uses [`recover_mutex`],
//!   [`recover_rwlock_read`], or [`recover_rwlock_write`]. Recovery accepts
//!   the inner value, clears the poison flag so later acquisitions behave
//!   normally, and emits one structured warning so the poisoning stays
//!   visible in telemetry without hot-path spam. Non-poisoned paths are
//!   unchanged.
//! - Invariant-sensitive state (for example plugin lifecycle transitions)
//!   must NOT use these helpers. It acquires fail-closed and maps
//!   poisoning to a typed error instead, so a torn transition can never be
//!   mistaken for a healthy one.
//!
//! Poison clearing uses `Mutex::clear_poison` / `RwLock::clear_poison`
//! (stabilized in Rust 1.77; the workspace MSRV is 1.88, so no fallback
//! is needed).

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Telemetry target for lock-poison recovery, matching the existing
/// `target:` convention (see `plugin.stderr`).
pub const LOCK_RECOVERY_TARGET: &str = "tiktools.lock";

/// Locks a mutex guarding recoverable state, accepting the inner value
/// when a previous holder panicked. The poison flag is cleared, so this
/// logs once per poisoning (not once per later acquisition); the guarded
/// data is kept. Never use this for invariant-sensitive transitions.
pub fn recover_mutex<'a, T>(lock: &'a Mutex<T>, name: &'static str) -> MutexGuard<'a, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: LOCK_RECOVERY_TARGET,
                lock = name,
                kind = "mutex",
                "recovered poisoned lock"
            );
            // The caller accepted recovery by choosing this helper, so the
            // sticky flag is cleared: later acquisitions behave normally.
            lock.clear_poison();
            poisoned.into_inner()
        }
    }
}

/// Reads an `RwLock` guarding recoverable state, accepting the inner value
/// when poisoned. Clears the poison flag; see [`recover_mutex`].
pub fn recover_rwlock_read<'a, T>(
    lock: &'a RwLock<T>,
    name: &'static str,
) -> RwLockReadGuard<'a, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: LOCK_RECOVERY_TARGET,
                lock = name,
                kind = "rwlock.read",
                "recovered poisoned lock"
            );
            lock.clear_poison();
            poisoned.into_inner()
        }
    }
}

/// Writes an `RwLock` guarding recoverable state, accepting the inner value
/// when poisoned. Clears the poison flag; see [`recover_mutex`].
pub fn recover_rwlock_write<'a, T>(
    lock: &'a RwLock<T>,
    name: &'static str,
) -> RwLockWriteGuard<'a, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: LOCK_RECOVERY_TARGET,
                lock = name,
                kind = "rwlock.write",
                "recovered poisoned lock"
            );
            lock.clear_poison();
            poisoned.into_inner()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poison_mutex(lock: &Mutex<u32>) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.lock().unwrap();
            panic!("test poison");
        }));
    }

    fn poison_rwlock(lock: &RwLock<u32>) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.write().unwrap();
            panic!("test poison");
        }));
    }

    #[test]
    fn healthy_locks_pass_through_unchanged() {
        let mutex = Mutex::new(7u32);
        assert_eq!(*recover_mutex(&mutex, "test mutex"), 7);
        let lock = RwLock::new(9u32);
        assert_eq!(*recover_rwlock_read(&lock, "test lock"), 9);
        *recover_rwlock_write(&lock, "test lock") = 10;
        assert_eq!(*recover_rwlock_read(&lock, "test lock"), 10);
    }

    #[test]
    fn poisoned_mutex_recovers_and_clears_the_poison_flag() {
        let mutex = Mutex::new(41u32);
        poison_mutex(&mutex);
        assert!(mutex.is_poisoned());
        assert_eq!(*recover_mutex(&mutex, "test mutex"), 41);
        // Recovery clears the sticky flag: later acquisitions behave
        // normally instead of re-reporting the same poisoning.
        assert!(!mutex.is_poisoned());
        assert!(mutex.lock().is_ok());
        *recover_mutex(&mutex, "test mutex") = 42;
        assert_eq!(*recover_mutex(&mutex, "test mutex"), 42);
        assert!(!mutex.is_poisoned());
    }

    #[test]
    fn poisoned_rwlock_recovers_and_clears_the_poison_flag() {
        let lock = RwLock::new(1u32);
        poison_rwlock(&lock);
        assert!(lock.is_poisoned());
        assert_eq!(*recover_rwlock_read(&lock, "test lock"), 1);
        assert!(!lock.is_poisoned());
        assert!(lock.read().is_ok());
        *recover_rwlock_write(&lock, "test lock") = 2;
        assert_eq!(*recover_rwlock_read(&lock, "test lock"), 2);
        assert!(!lock.is_poisoned());
        // A fresh poisoning still recovers (clearing is per-incident, not
        // a one-way latch that hides later panics).
        poison_rwlock(&lock);
        assert!(lock.is_poisoned());
        *recover_rwlock_write(&lock, "test lock") = 3;
        assert!(!lock.is_poisoned());
        assert_eq!(*recover_rwlock_read(&lock, "test lock"), 3);
    }
}
