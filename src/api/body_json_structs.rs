use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyServerActive {
    pub name: String,
    pub active: bool,
}
