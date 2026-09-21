//! Poison-tolerant helpers for host-side concurrency locks.
//!
//! Production locks guard runtime state shared across threads (plugin
//! registry, connection context, diagnostics). A panic while holding a lock
//! poisons it; historical code turned that into a host crash via
//! `expect(...)`. These helpers recover with `into_inner()` and emit one
//! structured warning so the host keeps serving while the poisoning stays
//! visible in telemetry. Non-poisoned paths are unchanged.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Telemetry target for lock-poison recovery, matching the existing
/// `target:` convention (see `plugin.stderr`).
pub const LOCK_RECOVERY_TARGET: &str = "tiktools.lock";

/// Locks a mutex, recovering the inner value when a previous holder
/// panicked. Recovery is logged once per call; the guarded data is kept.
pub fn mutex_or_recover<'a, T>(lock: &'a Mutex<T>, name: &'static str) -> MutexGuard<'a, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: LOCK_RECOVERY_TARGET,
                lock = name,
                kind = "mutex",
                "recovered poisoned lock"
            );
            poisoned.into_inner()
        }
    }
}

/// Reads an `RwLock`, recovering the inner value when poisoned.
pub fn read_or_recover<'a, T>(lock: &'a RwLock<T>, name: &'static str) -> RwLockReadGuard<'a, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: LOCK_RECOVERY_TARGET,
                lock = name,
                kind = "rwlock.read",
                "recovered poisoned lock"
            );
            poisoned.into_inner()
        }
    }
}

/// Writes an `RwLock`, recovering the inner value when poisoned.
pub fn write_or_recover<'a, T>(lock: &'a RwLock<T>, name: &'static str) -> RwLockWriteGuard<'a, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: LOCK_RECOVERY_TARGET,
                lock = name,
                kind = "rwlock.write",
                "recovered poisoned lock"
            );
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
        assert_eq!(*mutex_or_recover(&mutex, "test mutex"), 7);
        let lock = RwLock::new(9u32);
        assert_eq!(*read_or_recover(&lock, "test lock"), 9);
        *write_or_recover(&lock, "test lock") = 10;
        assert_eq!(*read_or_recover(&lock, "test lock"), 10);
    }

    #[test]
    fn poisoned_mutex_recovers_its_value() {
        let mutex = Mutex::new(41u32);
        poison_mutex(&mutex);
        assert!(mutex.is_poisoned());
        assert_eq!(*mutex_or_recover(&mutex, "test mutex"), 41);
        *mutex_or_recover(&mutex, "test mutex") = 42;
        assert_eq!(*mutex_or_recover(&mutex, "test mutex"), 42);
    }

    #[test]
    fn poisoned_rwlock_recovers_for_read_and_write() {
        let lock = RwLock::new(1u32);
        poison_rwlock(&lock);
        assert!(lock.is_poisoned());
        assert_eq!(*read_or_recover(&lock, "test lock"), 1);
        *write_or_recover(&lock, "test lock") = 2;
        assert_eq!(*read_or_recover(&lock, "test lock"), 2);
    }
}
