use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::structs::{BackendServer, ProxyConfig};

#[derive(Debug)]
pub struct ServerTracker {
    pub backends: HashMap<String, (Vec<BackendServer>, AtomicUsize)>,
}

impl ServerTracker {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /**
     * Warning: modifying this method could lead to a bottleneck
     */
    pub fn get_next_backend(&self, host: &str) -> Option<BackendServer> {
        // Get natural next backend
        self.backends.get(host).and_then(|(servers, idx)| {
            if servers.is_empty() {
                return None;
            }
            let next_idx = idx.fetch_add(1, Ordering::Relaxed);
            Some(servers[next_idx % servers.len()].clone())
        })
    }

    pub fn populate(&mut self, frontend_name: String, config: &ProxyConfig) {
        let cfg = config;
        // get backends
        let pool_lookup: HashMap<_, _> = cfg
            .pool_backends
            .iter()
            .map(|pb| (&pb.name, &pb.servers))
            .collect();
        // Process frontend
        let lookup_table = cfg
            // filter frontend on frontend_name
            .frontends
            .iter()
            .find(|f| f.name == frontend_name)
            // Get acls
            .into_iter()
            .flat_map(|frontend| &frontend.acls)
            // finally build tracker content
            .filter_map(|acl| {
                pool_lookup.get(&acl.backend).map(|servers| {
                    (
                        acl.host.clone(),
                        cfg.pool_servers
                            .iter()
                            // including only active servers
                            .filter(|server| servers.contains(&server.name) && server.active)
                            .cloned()
                            .collect::<Vec<_>>(),
                    )
                })
            })
            .collect::<Vec<_>>();
        for (host, backends) in lookup_table {
            self.backends
                .insert(host.clone(), (backends.clone(), AtomicUsize::new(0)));
        }
    }
}
