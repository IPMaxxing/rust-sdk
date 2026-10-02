use std::{
    any::Any,
    num::NonZeroUsize,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use lru::LruCache;

type Key = (&'static str, String);
type Entry = (Instant, Box<dyn Any + Send>);

pub(crate) struct Cache {
    ttl: Duration,
    entries: Mutex<LruCache<Key, Entry>>,
}

impl Cache {
    pub(crate) fn new(capacity: usize, ttl: Duration) -> Option<Self> {
        NonZeroUsize::new(capacity).map(|capacity| Self {
            ttl,
            entries: Mutex::new(LruCache::new(capacity)),
        })
    }

    pub(crate) fn get<T: Clone + 'static>(&self, product: &'static str, ip: &str) -> Option<T> {
        let key = (product, ip.to_owned());
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let (stored_at, value) = entries.get(&key)?;
        if stored_at.elapsed() < self.ttl {
            return value.downcast_ref::<T>().cloned();
        }
        entries.pop(&key);
        None
    }

    pub(crate) fn put<T: Send + 'static>(&self, product: &'static str, ip: &str, value: T) {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .put((product, ip.to_owned()), (Instant::now(), Box::new(value)));
    }
}
