use clap::Parser;
use log::{debug, error, info};
use std::{collections::HashMap, env, fs::File, path::PathBuf, sync::Arc};

use crate::{
    api::body_json_structs::BodyBackendPost,
    constants::{DEFAULT_CONFIG_PATH, DEFAULT_TLS_CERT_PATH},
    forwarders::backend::Backend,
    structs::{AclConfig, FrontEnd, GenericError, ProxyConfig},
};

// Define the CLI arguments structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    // Path to the config file
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    // Path to the config file
    #[arg(short = 't', long)]
    tls_certs_path: Option<PathBuf>,

    // Listening API Rest port
    #[arg(short = 'p', long)]
    api_port: Option<u16>,

    //Listening API Rest addr
    #[arg(short = 'a', long)]
    api_addr: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ConfigManager {
    config_path: PathBuf,
    tls_certs_path: PathBuf,
    config: Option<Arc<ProxyConfig>>,
    trackers: HashMap<String, Arc<Backend>>,
}

impl ConfigManager {
    pub fn new(clap_args: Args) -> Self {
        let config_path = clap_args
            .config
            .or_else(|| env::var("CONFIG_PATH").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

        let tls_certs_path = clap_args
            .tls_certs_path
            .or_else(|| env::var("DEFAULT_TLS_CERT_PATH").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TLS_CERT_PATH));

        Self {
            config_path,
            tls_certs_path,
            config: None,
            trackers: HashMap::new(),
        }
    }

    pub async fn load(&mut self) -> Result<(), GenericError> {
        info!("Configuration file path: {:?}", self.config_path.clone());
        let file = File::open(self.config_path.clone())?;

        // Load config
        let config_proxy: ProxyConfig = serde_yaml::from_reader(file)?;
        self.config = Some(Arc::new(config_proxy.clone()));
        // check config
        // Load tracker by frontend
        self.load_servers_tracker().await;
        Ok(())
    }

    async fn load_servers_tracker(&mut self) {
        // Load trackers by frontend
        let config = self.get_config().await;
        for frontend in config.frontends.clone() {
            let servers_tracker = {
                let mut tracker = Backend::new();
                tracker.populate(frontend.clone().name, &self.get_config().await.clone());
                Arc::new(tracker)
            };
            self.trackers.insert(frontend.name.clone(), servers_tracker);
        }
    }

    pub fn get_tracker(&self, frontend_name: String) -> Option<Arc<Backend>> {
        self.trackers.get(frontend_name.as_str()).cloned()
    }

    pub fn get_frontends(&self) -> Vec<FrontEnd> {
        self.config.clone().unwrap().frontends.clone()
    }

    pub async fn get_config_tls_certs_path(&self) -> PathBuf {
        self.tls_certs_path.clone()
    }

    pub async fn get_config(&self) -> Arc<ProxyConfig> {
        self.config.clone().unwrap()
    }

    /**
     * method identpotent
     */
    pub async fn set_backend(
        &mut self,
        new_backend: BodyBackendPost,
    ) -> Result<Vec<String>, GenericError> {
        debug!("BodyBackendPost: {:?}", new_backend);
        let current_config = self.get_config().await;
        let mut new_config = (*current_config).clone();

        // Track changes
        let mut changes: Vec<String> = Vec::new();

        // Update ACLs for relevant frontends
        for frontend in &mut new_config.frontends {
            if new_backend.frontends.contains(&frontend.name) {
                let acl_exists = frontend
                    .acls
                    .iter_mut()
                    .find(|x| x.domain == new_backend.domain);

                let new_acl = AclConfig {
                    antibot: Some(new_backend.antibot.clone()),
                    name: new_backend.name.clone(),
                    backend: new_backend.name.clone(),
                    domain: new_backend.domain.clone(),
                };

                match acl_exists {
                    Some(existing_acl) => {
                        *existing_acl = new_acl;
                        changes.push(format!("acl changed : {}", new_backend.name));
                    }
                    None => {
                        frontend.acls.push(new_acl);
                        changes.push(format!("acl added : {}", new_backend.name));
                    }
                }
            }
        }

        // Check if backend exists
        if current_config
            .pool_backends
            .iter()
            .any(|item| item.name == new_backend.name)
        {
            changes.push(format!("backend changed : {}", new_backend.name));
        } else {
            changes.push(format!("backend add : {}", new_backend.name));
        }

        // Filter out existing backend and add the new one
        let mut new_pool_backends: Vec<_> = current_config
            .pool_backends
            .iter()
            .filter(|b| b.name != new_backend.name)
            .cloned()
            .collect();

        let backend = crate::structs::Backend {
            name: new_backend.name.clone(),
            servers: new_backend.servers.iter().map(|s| s.name.clone()).collect(),
        };
        new_pool_backends.push(backend);

        // Process server changes
        let (changed_servers, new_servers): (Vec<_>, Vec<_>) =
            new_backend.servers.into_iter().partition(|item| {
                current_config
                    .pool_servers
                    .iter()
                    .any(|s| s.name == item.name)
            });

        for item in &changed_servers {
            changes.push(format!("Server changed: {}", item.name));
        }
        for item in &new_servers {
            changes.push(format!("Server added: {}", item.name));
        }

        // Build final server list
        let unchanged_servers: Vec<_> = current_config
            .pool_servers
            .iter()
            .filter(|s| !changed_servers.iter().any(|cs| cs.name == s.name))
            .cloned()
            .collect();

        // Update config
        new_config.pool_servers = unchanged_servers
            .into_iter()
            .chain(changed_servers)
            .chain(new_servers)
            .collect();

        new_config.pool_backends = new_pool_backends;

        self.set_config(new_config.into()).await;
        println!("changes: {:?}", changes);
        Ok(changes)
    }

    pub async fn set_server_enabled_state(
        &mut self,
        server_name: String,
        enabled: bool,
    ) -> Option<bool> {
        if let Some(config) = &self.config {
            let mut new_servers = config.pool_servers.clone();
            let mut updated = false;

            for server in &mut new_servers {
                if server.name == server_name {
                    server.enabled = enabled;
                    updated = true;
                    break; // No need to continue once found
                }
            }

            if updated {
                // Update config
                let mut config = (**config).clone();
                config.pool_servers = new_servers;
                let _ = &self.set_config(config.into()).await;
                Some(true)
            } else {
                info!("Server '{}' not found in configuration", server_name);
                None
            }
        } else {
            error!("No configuration loaded");
            None
        }
    }

    pub async fn set_config(&mut self, new_config: Arc<ProxyConfig>) {
        // Store back as Arc
        self.config = Some(new_config);
        // updating trackers
        self.load_servers_tracker().await;
    }
}
