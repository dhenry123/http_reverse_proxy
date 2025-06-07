use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::debug;

use crate::structs::BackendServer;

#[derive(Debug)]
pub struct RuntimeBackends {
    disabled: Arc<RwLock<HashMap<String, BackendServer>>>,
}

impl RuntimeBackends {
    pub fn new(health_check_interval: Duration) -> Arc<Self> {
        let disabled = Arc::new(RwLock::new(HashMap::new()));
        let instance = Arc::new(Self { disabled });

        let instance_clone = instance.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(health_check_interval);
            loop {
                interval.tick().await;

                // Phase 1: Collect all backends to check
                let backends_to_check = {
                    let guard = instance_clone.disabled.read().await;
                    guard
                        .iter()
                        .map(|(name, backend)| (name.clone(), backend.clone()))
                        .collect::<Vec<_>>()
                };

                // Phase 2: Check connectivity (concurrently)
                let mut checks = Vec::new();
                for (name, backend) in backends_to_check {
                    let target = format!("{}:{}", backend.host, backend.port);
                    checks.push(async move {
                        let is_online = Self::check_backend(backend.name, target).await;
                        (name, is_online)
                    });
                }

                // Phase 3: Process results
                let results = futures::future::join_all(checks).await;
                let mut write_guard = instance_clone.disabled.write().await;
                for (name, is_online) in results {
                    if is_online {
                        debug!("Removing server: {} from disabled list", &name);
                        write_guard.remove(&name);
                    }
                }
            }
        });

        instance
    }

    async fn check_backend(backend_name: String, target: String) -> bool {
        debug!("Checking backend {} - server {}", backend_name, target);
        match timeout(Duration::from_millis(1000), TcpStream::connect(&target)).await {
            Ok(Ok(_)) => {
                info!("Backend '{}' ({}) is back online", backend_name, target);
                return true;
            }
            Ok(Err(e)) => {
                info!("Backend '{}' still unreachable [{}]", backend_name, e);
                return false;
            }
            Err(_) => {
                debug!("Timeout after 500ms");
                return false;
            }
        }
    }

    pub async fn disable_backend(&self, server: BackendServer) {
        self.disabled
            .write()
            .await
            .insert(server.name.clone(), server);
    }

    pub async fn is_disabled(&self, backend_name: &str) -> bool {
        self.disabled.read().await.contains_key(backend_name)
    }

    /**
     * List of backend name disabled
     */
    pub async fn list_backend_name_disabled(&self) -> Vec<String> {
        self.disabled.read().await.clone().into_keys().collect()
    }
}
