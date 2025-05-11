use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocols {
    Http,
    Tcp,
}
// Castable to &str
impl AsRef<str> for ProxyProtocols {
    fn as_ref(&self) -> &str {
        match self {
            ProxyProtocols::Http => "http",
            ProxyProtocols::Tcp => "tcp",
        }
    }
}

// Acl config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclConfig {
    pub name: String,
    pub host: String,
    pub backend: String,
    pub antibot: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontEnd {
    pub name: String,
    pub protocol: ProxyProtocols,
    pub port: u16,
    pub addr: String,
    pub tls: bool,
    pub active: bool,
    pub acls: Vec<AclConfig>,
}

// Backend server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    pub name: String,
    pub servers: Vec<String>,
}

// Backend server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendServer {
    pub name: String,
    pub host: String, // fqdn | ip address
    pub port: u16,
    pub protocol: ProxyProtocols,
    pub tls: bool, // final endpoing is ssl ???
    pub active: bool,
    pub path: Option<String>,
}

// Default value function
const fn default_version() -> u64 {
    0 // Your default value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub frontends: Vec<FrontEnd>,
    pub pool_backends: Vec<Backend>,
    pub pool_servers: Vec<BackendServer>,
    #[serde(default = "default_version")]
    pub version: u64,
}

pub type GenericError = Box<dyn Error + Send + Sync + 'static>;
pub type GenericResult<T> = Result<T, GenericError>;
