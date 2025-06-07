use clap::Parser;
use log::{error, info};
use std::{collections::HashMap, env, fs::File, path::PathBuf, sync::Arc};

use crate::{
    constants::{DEFAULT_CONFIG_PATH, DEFAULT_TLS_CERT_PATH},
    forwarders::backend::Backend,
    structs::{FrontEnd, GenericError, ProxyConfig},
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

    pub async fn set_server_active_state(
        &mut self,
        server_name: String,
        active: bool,
    ) -> Option<bool> {
        if let Some(config) = &self.config {
            let mut new_servers = config.pool_servers.clone();
            let mut updated = false;

            for server in &mut new_servers {
                if server.name == server_name {
                    server.active = active;
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
