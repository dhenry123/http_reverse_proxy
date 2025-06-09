use serde::{Deserialize, Serialize};

use crate::structs::BackendServer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyServerEnabled {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyBackendPost {
    pub name: String,
    pub domain: String,
    pub antibot: bool,
    pub frontends: Vec<String>,
    pub servers: Vec<BackendServer>,
}
