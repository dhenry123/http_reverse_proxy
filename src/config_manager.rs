use arc_swap::ArcSwap;
use clap::Parser;
use std::{env, fs::File, path::PathBuf, sync::Arc};

use crate::{
    constants::{DEFAULT_CONFIG_PATH, DEFAULT_TLS_CERT_PATH},
    structs::{GenericError, ProxyConfig},
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

pub struct ConfigManager {
    config_path: PathBuf,
    tls_certs_path: PathBuf,
    config: Option<Arc<ArcSwap<ProxyConfig>>>,
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
        }
    }

    pub async fn load(&mut self) -> Result<(), GenericError> {
        println!("Configuration file path: {:?}", self.config_path.clone());
        let file = File::open(self.config_path.clone())?;

        let config: ProxyConfig = serde_yaml::from_reader(file)?;
        self.config = Some(Arc::new(ArcSwap::new(Arc::new(config))));
        Ok(())
    }

    pub async fn get_config_tls_certs_path(&self) -> PathBuf {
        self.tls_certs_path.clone()
    }

    pub async fn get_config(&self) -> Arc<ArcSwap<ProxyConfig>> {
        self.config.clone().unwrap()
    }
}
