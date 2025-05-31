use crate::constants::{API_LISTENING_PORT, API_SERVERS_ACTIVE};

pub fn get_api_server_active_set() -> String {
    format!(
        "http://127.0.0.1:{}/{}",
        API_LISTENING_PORT, API_SERVERS_ACTIVE,
    )
}
