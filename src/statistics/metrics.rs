// src/metrics.rs
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct ProxyMetrics {
    // Counters for each frontend (by name)
    pub frontend_hits: Arc<DashMap<String, AtomicU64>>,
    // Counters for each domain (by name)
    pub domain_hits: Arc<DashMap<String, AtomicU64>>,
    // Counters for each backend (by name)
    pub backend_hits: Arc<DashMap<String, AtomicU64>>,
    // Counters for each server (by name)
    pub server_hits: Arc<DashMap<String, AtomicU64>>,
}

impl ProxyMetrics {
    pub fn new() -> Self {
        Self {
            frontend_hits: Arc::new(DashMap::new()),
            domain_hits: Arc::new(DashMap::new()),
            backend_hits: Arc::new(DashMap::new()),
            server_hits: Arc::new(DashMap::new()),
        }
    }

    pub fn increment_frontend(&self, name: &str) {
        self.increment_counter(&self.frontend_hits, name);
    }

    pub fn increment_domain(&self, name: &str) {
        self.increment_counter(&self.domain_hits, name);
    }

    pub fn increment_backend(&self, name: &str) {
        self.increment_counter(&self.backend_hits, name);
    }

    pub fn increment_server(&self, name: &str) {
        self.increment_counter(&self.server_hits, name);
    }

    fn increment_counter(&self, map: &DashMap<String, AtomicU64>, key: &str) {
        map.entry(key.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    // to read counters
    pub fn get_frontend_count(&self, frontend_name: &str) -> Option<u64> {
        self.frontend_hits
            .get(frontend_name)
            .map(|v| v.load(Ordering::Relaxed))
    }

    pub fn get_domain_count(&self, name: &str) -> Option<u64> {
        self.domain_hits
            .get(name)
            .map(|v| v.load(Ordering::Relaxed))
    }

    pub fn get_backend_count(&self, name: &str) -> Option<u64> {
        self.backend_hits
            .get(name)
            .map(|v| v.load(Ordering::Relaxed))
    }

    pub fn get_server_count(&self, name: &str) -> Option<u64> {
        self.server_hits
            .get(name)
            .map(|v| v.load(Ordering::Relaxed))
    }
}
