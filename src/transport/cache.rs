use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple in-memory LRU cache with TTL.
pub struct ResponseCache {
    inner: Mutex<LruCache<String, CacheEntry>>,
    ttl: Duration,
}

struct CacheEntry {
    data: String,
    inserted_at: Instant,
}

impl ResponseCache {
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(
                NonZeroUsize::new(max_entries.max(1)).unwrap(),
            )),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut cache = self.inner.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.data.clone());
            }
            // Expired — remove it
            cache.pop(key);
        }
        None
    }

    pub fn insert(&self, key: String, data: String) {
        let mut cache = self.inner.lock().unwrap();
        cache.put(
            key,
            CacheEntry {
                data,
                inserted_at: Instant::now(),
            },
        );
    }
}
