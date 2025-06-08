use serde::{Deserialize, Serialize};

use crate::structs::BackendServer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyServerActive {
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyBackendPost {
    pub name: String,
    pub domain: String,
    pub frontends: Vec<String>,
    pub servers: Vec<BackendServer>,
}
