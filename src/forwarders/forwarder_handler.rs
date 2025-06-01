use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    HeaderMap, Method, Request, Response, Uri,
    body::{self, Incoming},
    header::HeaderValue,
};

use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, Error, connect::HttpConnector},
    rt::TokioExecutor,
};
use log::debug;
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::http;

use crate::{
    api::api_helper::get_api_server_active_set,
    config_manager::ConfigManager,
    constants::{
        HTTP_HEADER_X_FORWARDED_FOR, HTTP_HEADER_X_REAL_IP, INTERNAL_ROUTE_ANTIBOT,
        INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,
    },
    forwarders::{
        forwarder_helper::{get_upstream_server, is_domain_configured_for_antibot},
        forwarder_ws::handle_websocket_upgrade,
    },
    internal_server_free_port,
    structs::BackendServer,
};

use super::{
    forwarder_helper::{build_upstream_uri, is_cookie_antibot, is_websocket_request},
    servers_tracker::ServerTracker,
};

enum FallBackResponseType {
    ServerUnavailable,
}
/**
 * Alter output header client->listener (Response)
 */
pub async fn set_response_header(original_host: String, response: &mut Response<Incoming>) {
    // Handle redirect responses (301, 302, etc.)
    if response.status().is_redirection() {
        if let Some(location) = response.headers().get(hyper::header::LOCATION) {
            if let Ok(location_str) = location.to_str() {
                if let Ok(location_uri) = location_str.parse::<Uri>() {
                    // Check if the URI is absolute by looking for scheme
                    if location_uri.scheme_str().is_some() {
                        if original_host != "" {
                            // Get path and query
                            let path_and_query = location_uri
                                .path_and_query()
                                .map(|pq| pq.as_str())
                                .unwrap_or("/");

                            // Rebuild URI with original proxy host/scheme
                            let new_uri = format!("https://{}{}", original_host, path_and_query);
                            if let Ok(new_uri) = new_uri.parse::<Uri>() {
                                response.headers_mut().insert(
                                    hyper::header::LOCATION,
                                    new_uri.to_string().parse().unwrap(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    peer_addr: SocketAddr,
    frontend_name: String,
    servers_tracker: Arc<ServerTracker>,
    config: Arc<RwLock<ConfigManager>>,
    client: Client<HttpsConnector<HttpConnector>, Incoming>,
) -> Result<Response<body::Incoming>, hyper_util::client::legacy::Error> {
    if is_websocket_request(&req) {
        debug!("websocket request detected");
        return handle_websocket_upgrade(req, &servers_tracker).await;
    }

    let (parts, body) = req.into_parts();

    // Capture the original host and scheme for redirect rewriting
    let original_host = parts
        .headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap(); // Convert to &str safely

    // Prepare antibot
    let is_antibot_protected = is_domain_configured_for_antibot(
        frontend_name.clone(),
        original_host.clone(),
        config.clone(),
    )
    .await;

    // upstream uri - server selected (will be desactived if server not available)
    let upstream_server = get_upstream_server(original_host.clone(), &servers_tracker);
    let mut upstream_uri = match upstream_server.clone() {
        Some(server) => build_upstream_uri(server, false),
        None => "".to_string(),
    };
    if upstream_uri == "" {
        // Internal server - No server available
        upstream_uri = get_internal_error_no_backend_server_available_uri(parts.clone());
    } else {
        // antibot for this host ?
        if is_antibot_protected {
            if !is_cookie_antibot(parts.headers.get("cookie")) {
                upstream_uri = get_internal_antibot_uri();
            }
        }
        upstream_uri = format!("{}{}", upstream_uri, parts.uri.to_string());
    }
    let uri = upstream_uri.parse::<Uri>();
    let upstream_uri: Uri;
    debug!("original_host {:?}", original_host);

    match uri {
        Ok(uri) => upstream_uri = uri,
        Err(initial_error) => {
            debug!("Initial error: {}", initial_error);
            debug!("Parts: {:?}", parts);
            debug!("original_host {:?}", original_host);
            let upstream_uri = get_internal_error_no_backend_server_available_uri(parts.clone());
            let upstream_uri = upstream_uri.parse::<Uri>().unwrap();
            let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();
            let response = client.get(upstream_uri).await;
            match response {
                Ok(mut response) => {
                    let original_host = original_host.clone();
                    set_response_header(original_host, &mut response).await;
                    return Ok::<Response<body::Incoming>, hyper_util::client::legacy::Error>(
                        response,
                    );
                }
                Err(internal_server_error) => {
                    log::error!(
                        "Request forwarding calling internal server, error: {:?}",
                        internal_server_error
                    );
                    return Err(internal_server_error);
                }
            }
        }
    }
    //====> To check round robin load balance
    debug!("upstream_uri: {}", upstream_uri);

    let forwarded_req = get_forwarded_red(parts.clone(), upstream_uri.clone(), peer_addr, body);

    let response = client.request(forwarded_req).await;

    match response {
        Ok(mut response) => {
            // replace backend host response with original host
            let original_host = original_host.clone();
            set_response_header(original_host, &mut response).await;
            debug!("{:?}", response);
            Ok::<Response<body::Incoming>, hyper_util::client::legacy::Error>(response)
        }
        Err(initial_error) => {
            log::error!(
                "Request forwarding initial error: {:?} - upstream uri: {}",
                initial_error,
                upstream_uri
            );
            if initial_error.is_connect() {
                // upstream serveur failure, server must be desactivated
                log::error!("upstream_server failure: {:?}", upstream_server);
                deactivate_server(upstream_server).await;
                // Return internal response unavailable service 503
                match get_fallback_response(parts.clone(), FallBackResponseType::ServerUnavailable)
                    .await
                {
                    Ok(mut response) => {
                        let original_host = original_host.clone();
                        set_response_header(original_host, &mut response).await;
                        Ok::<Response<body::Incoming>, hyper_util::client::legacy::Error>(response)
                    }
                    Err(internal_server_error) => {
                        log::error!(
                            "Error on calling fallback response: {:?}",
                            internal_server_error
                        );
                        Err(initial_error)
                    }
                }
            } else {
                Err(initial_error)
            }
        }
    }
}

/**
 * deactivate server
 */
async fn deactivate_server(upstream_server: Option<BackendServer>) {
    if upstream_server.is_some() {
        let upstream_uri = get_api_server_active_set();
        let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();
        let url = upstream_uri.parse::<Uri>().unwrap();
        let authority = url.authority().unwrap().clone();
        let json: String = json!({
            "name": upstream_server.unwrap().name,
            "active": false
        })
        .to_string();

        let body = Full::new(Bytes::from(json));
        let req = Request::builder()
            .uri(url)
            .header(hyper::header::HOST, authority.as_str())
            .method(Method::PUT)
            .body(body)
            .unwrap();
        let _ = client.request(req).await;
    }
}

async fn get_fallback_response(
    parts: http::request::Parts,
    _all_back_response_type: FallBackResponseType,
) -> Result<Response<Incoming>, Error> {
    let upstream_uri = get_internal_error_no_backend_server_available_uri(parts);
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();
    client.get(upstream_uri.parse::<Uri>().unwrap()).await
}

fn get_internal_antibot_uri() -> String {
    format!(
        "http://127.0.0.1:{}/{}",
        internal_server_free_port::get_global_port(),
        INTERNAL_ROUTE_ANTIBOT,
    )
}

fn get_internal_error_no_backend_server_available_uri(parts: http::request::Parts) -> String {
    format!(
        "http://127.0.0.1:{}/{}{}",
        internal_server_free_port::get_global_port(),
        INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,
        parts.uri.to_string()
    )
}

/**
 * Build forwarded request with all original headers
 */
fn get_forwarded_red(
    parts: http::request::Parts,
    upstream_uri: Uri,
    peer_addr: SocketAddr,
    body: Incoming,
) -> hyper::Request<hyper::body::Incoming> {
    let mut builder = Request::builder().method(parts.method).uri(upstream_uri);

    // Copy all headers from original request
    for (name, value) in parts.headers.iter() {
        builder = builder.header(name, value);
    }

    // Add X-forwarded-for headers
    let peer_ip_as_string = peer_addr.ip().to_string();
    let peer_as_str = peer_ip_as_string.as_str();
    let mut headers_map = HeaderMap::new();
    headers_map.append(
        HTTP_HEADER_X_FORWARDED_FOR,
        HeaderValue::from_str(peer_as_str).unwrap(),
    );
    headers_map.append(
        HTTP_HEADER_X_REAL_IP,
        HeaderValue::from_str(peer_as_str).unwrap(),
    );
    let _ = builder.headers_mut().insert(&mut headers_map);

    // Body
    builder.body(body).unwrap()
}
