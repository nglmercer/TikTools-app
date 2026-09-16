//! FIFO-bounded cache for repeated analysis results.
//!
//! Chat repeats the same messages and nicknames constantly; caching by the
//! full input text (plus a settings-digest generation owned by the caller)
//! keeps steady-state enrichment allocation-free on hits. Eviction is plain
//! FIFO: chat locality is recency-heavy but the caps are generous enough that
//! LRU bookkeeping would cost more than it saves.

use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    hash::Hash,
};

/// FIFO-bounded map. Refreshing an existing key replaces its value without
/// duplicating its queue entry; inserts beyond capacity evict oldest first.
#[derive(Debug, Default)]
pub struct BoundedCache<K = String, V = serde_json::Value> {
    map: HashMap<K, V>,
    order: VecDeque<K>,
    cap: usize,
}

impl<K, V> BoundedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.map.get(key).cloned()
    }

    pub fn insert(&mut self, key: K, value: V) {
        // Refreshing an existing key must not duplicate its order entry.
        if let Some(slot) = self.map.get_mut(&key) {
            *slot = value;
            return;
        }
        while self.map.len() >= self.cap {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn evicts_oldest_first() {
        let mut cache = BoundedCache::new(2);
        cache.insert("a".to_owned(), Value::from(1));
        cache.insert("b".to_owned(), Value::from(2));
        cache.insert("c".to_owned(), Value::from(3));
        assert!(cache.get("a").is_none());
        assert_eq!(cache.get("b"), Some(Value::from(2)));
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn refresh_does_not_duplicate_eviction_order() {
        let mut cache = BoundedCache::new(2);
        cache.insert("a".to_owned(), Value::from(1));
        cache.insert("b".to_owned(), Value::from(2));
        cache.insert("a".to_owned(), Value::from(10));
        cache.insert("c".to_owned(), Value::from(3));
        // `a` was refreshed, not re-queued, so it is still the oldest entry.
        assert!(cache.get("a").is_none());
        assert_eq!(cache.get("b"), Some(Value::from(2)));
        assert_eq!(cache.get("c"), Some(Value::from(3)));
    }

    #[test]
    fn generic_keys_work_for_non_string_values() {
        let mut cache: BoundedCache<u64, String> = BoundedCache::new(1);
        cache.insert(7, "seven".to_owned());
        assert_eq!(cache.get(&7), Some("seven".to_owned()));
    }
}
