use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::url::HttpUrl;

#[derive(Default)]
pub struct RequestCache {
    cache: HashMap<CacheKey, CacheEntry>,
}

impl RequestCache {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, http_url: &HttpUrl) -> Option<&str> {
        self.cache.get(&http_url.into()).and_then(|entry| {
            if entry.is_stale() {
                None
            } else {
                Some(entry.response.as_str())
            }
        })
    }

    pub fn set(&mut self, http_url: &HttpUrl, response: &str, max_age: Option<u64>) {
        self.cache.insert(
            http_url.into(),
            CacheEntry {
                response: response.to_string(),
                max_age: max_age.map(Duration::from_secs),
                fetched_at: Instant::now(),
            },
        );
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct CacheKey(String);

impl From<&HttpUrl> for CacheKey {
    fn from(http_url: &HttpUrl) -> Self {
        Self(format!(
            "http{}://{}:{}{}",
            if http_url.tls { "s" } else { "" },
            http_url.host,
            http_url.port,
            http_url.path
        ))
    }
}

struct CacheEntry {
    response: String,
    max_age: Option<Duration>,
    fetched_at: Instant,
}

impl CacheEntry {
    pub fn is_stale(&self) -> bool {
        if let Some(max_age) = self.max_age {
            self.fetched_at.elapsed() > max_age
        } else {
            false
        }
    }
}
