use std::{sync::Arc, time::Duration};

use hyper::{Request, body, header::HeaderValue};
use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::{TokioExecutor, TokioTimer},
};
use log::debug;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    config_manager::ConfigManager,
    constants::{ANTIBOT_COOKIE_NAME, POOL_IDLE_TIMEOUT, POOL_MAX_IDLE_PER_HOST},
    structs::BackendServer,
};

use super::backend::Backend;
use cookie::Cookie;

pub fn build_upstream_uri(backend_server: BackendServer, is_web_socket: bool) -> String {
    let mut upstream: String;
    // protocol
    let proto = if is_web_socket {
        "ws"
    } else {
        backend_server.protocol.as_ref()
    };
    // scheme
    if backend_server.tls {
        upstream = format!("{}s://{}", proto, backend_server.host);
    } else {
        upstream = format!("{}://{}", proto, backend_server.host);
    }
    //port
    upstream = format!("{}:{}", upstream, backend_server.port);
    // Optional path
    if backend_server.path.is_some() {
        upstream = format!("{}:{}", upstream, backend_server.path.clone().unwrap());
    }
    return upstream;
}

/**
 * return an http connector
 */
pub fn get_http_client() -> Client<hyper_tls::HttpsConnector<HttpConnector>, body::Incoming> {
    let mut http_connector = HttpConnector::new();
    //http_connector.set_nodelay(true);
    http_connector.set_nodelay(true);
    http_connector.set_reuse_address(true);
    http_connector.set_keepalive(Some(std::time::Duration::from_secs(60)));

    let https_connector = HttpsConnector::new();

    Client::builder(TokioExecutor::new())
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT))
        .http1_preserve_header_case(true)
        .pool_timer(TokioTimer::new())
        .pool_idle_timeout(Duration::from_secs(60))
        .build::<_, body::Incoming>(https_connector)
}

/**
 * return the final uri selecting the backend with roundrobin
 */
pub fn get_upstream_server(
    original_host: String,
    servers_tracker: &Arc<Backend>,
) -> Option<BackendServer> {
    // Which backend ?
    let backend_server = servers_tracker.get_next_backend(&original_host);
    debug!("backend_server: {:?}", backend_server);
    backend_server
}

/**
 * Browser config to look for frontend/host is set with antibot
 */
pub async fn is_domain_configured_for_antibot(
    frontend_name: String,
    original_host: String,
    config_manager: Arc<RwLock<ConfigManager>>,
) -> bool {
    let lookup_table = config_manager
        // filter frontend on frontend_name
        .read()
        .await
        .get_frontends()
        .into_iter()
        .find(|f| f.name == frontend_name)
        // Get acls
        .into_iter()
        .flat_map(|frontend| frontend.acls)
        .find(|a| a.domain == original_host)
        .into_iter()
        .collect::<Vec<_>>();

    if lookup_table.len() == 1 {
        let antibot_state = lookup_table.get(0).unwrap().antibot;
        if antibot_state.is_some() && antibot_state.unwrap() {
            //println!("Configured with antibot");
            return true;
        }
    }
    // println!("Not configured with antibot");
    return false;
}

/**
 * Simple cookie...
 */
pub fn get_cookie_antibot(host: String) -> Cookie<'static> {
    let cookie = Cookie::build((ANTIBOT_COOKIE_NAME, Uuid::new_v4().to_string()))
        .domain(host)
        .path("/")
        .secure(false)
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .max_age(cookie::time::Duration::hours(2))
        .build();
    return cookie;
}

pub fn is_cookie_antibot(cookie_http_header: Option<&HeaderValue>) -> bool {
    if cookie_http_header.is_none() {
        return false;
    } else {
        // check antibot cookie
        for cookie in Cookie::split_parse(cookie_http_header.unwrap().to_str().unwrap()) {
            let cookie = cookie;
            if cookie.is_ok() && cookie.unwrap().name() == ANTIBOT_COOKIE_NAME {
                return true;
            }
        }
        return false;
    }
}

// Helper function to check WebSocket request
pub fn is_websocket_request(req: &Request<hyper::body::Incoming>) -> bool {
    req.headers()
        .get("Upgrade")
        .and_then(|h| h.to_str().ok())
        .map(|h| h.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}
