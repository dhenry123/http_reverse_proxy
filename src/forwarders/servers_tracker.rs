use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use arc_swap::ArcSwap;

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

    pub fn get_next_backend(&self, host: &str) -> Option<BackendServer> {
        // Get natural next backend
        let final_server = self.backends.get(host).map(|(servers, idx)| {
            let next_idx = idx.fetch_add(1, Ordering::Relaxed);
            let server = servers[next_idx % servers.len()].clone();
            server
        });
        // if server, check is active?
        if final_server.is_some() && !final_server.clone().unwrap().active {
            None
        } else {
            final_server
        }
    }

    // pub fn get_first_backend(&self, host: &str) -> Option<BackendServer> {
    //     self.backends.get(host).and_then(|(servers, _)| {
    //         servers.first().cloned() // Always returns first server
    //     })
    // }

    /**
     * Build structure servers_tracker
     * Server tracker table is set per frontend_name
     * ([domain/path],[server1,server2,...]
     */
    pub fn populate(&mut self, frontend_name: String, config: Arc<ArcSwap<ProxyConfig>>) {
        let cfg = config.load().clone();
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
                            .filter(|server| servers.contains(&server.name))
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
