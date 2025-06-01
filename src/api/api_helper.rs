use std::collections::HashMap;

use crate::constants::{API_LISTENING_PORT, API_SERVERS_ACTIVE, API_VERSION};

pub fn get_api_server_active_set() -> String {
    format!(
        "http://127.0.0.1:{}/{}/{}",
        API_LISTENING_PORT, API_VERSION, API_SERVERS_ACTIVE,
    )
}

pub fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Some((key.to_owned(), value.to_owned())),
                _ => None,
            }
        })
        .collect()
}
