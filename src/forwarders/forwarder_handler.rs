use arc_swap::ArcSwap;
use hyper::{
    HeaderMap, Request, Response, Uri,
    body::{self, Incoming},
    header::HeaderValue,
};

use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use std::{net::SocketAddr, sync::Arc};

use crate::{
    constants::{
        HTTP_HEADER_X_FORWARDED_FOR, HTTP_HEADER_X_REAL_IP, HTTP_INTERNAL_SERVER,
        INTERNAL_ROUTE_ANTIBOT, INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,
    },
    forwarders::{
        forwarder_helper::{get_upstream_uri, is_domain_configured_for_antibot},
        forwarder_ws::handle_websocket_upgrade,
    },
    structs::ProxyConfig,
};

use super::{
    forwarder_helper::{is_cookie_antibot, is_websocket_request},
    servers_tracker::ServerTracker,
};

/**
 * Alter output header client->listener (Response)
 */
pub async fn set_response_header(original_host: String, response: &mut Response<Incoming>) {
    //println!("Backend response: {:?}", response);
    // Handle redirect responses (301, 302, etc.)
    if response.status().is_redirection() {
        //println!("Redirection detected: {:?}", response);
        if let Some(location) = response.headers().get(hyper::header::LOCATION) {
            //println!("location detected: {:?}", location);
            if let Ok(location_str) = location.to_str() {
                if let Ok(location_uri) = location_str.parse::<Uri>() {
                    //println!("location_uri: {:?}", location_uri);
                    // Check if the URI is absolute by looking for scheme
                    if location_uri.scheme_str().is_some() {
                        //println!("scheme_str ok");
                        if original_host != "" {
                            // Get path and query
                            let path_and_query = location_uri
                                .path_and_query()
                                .map(|pq| pq.as_str())
                                .unwrap_or("/");

                            //println!("path_and_query: {:?}", path_and_query);
                            // Rebuild URI with original proxy host/scheme
                            let new_uri = format!("https://{}{}", original_host, path_and_query);
                            //println!("new_uri: {:?}", new_uri);
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
) -> Result<Response<body::Incoming>, hyper_util::client::legacy::Error> {
    // peer address:port
    let peer_addr = req.extensions().get::<SocketAddr>().cloned().unwrap();
    // println!(peer_addr: "{:?}", peer_addr);

    let frontend_name = req.extensions().get::<String>().cloned().unwrap();
    // println!("frontend_name: {:?}", frontend_name);

    let servers_tracker = req
        .extensions()
        .get::<Arc<arc_swap::ArcSwapAny<Arc<ServerTracker>>>>()
        .cloned()
        .unwrap()
        .clone();
    //println!("servers_tracker: {:?}", servers_tracker);

    let config = req
        .extensions()
        .get::<Arc<ArcSwap<ProxyConfig>>>()
        .cloned()
        .unwrap()
        .clone();
    //println!("config: {:?}", config);

    if is_websocket_request(&req) {
        println!("websocket request detected");
        return handle_websocket_upgrade(req, servers_tracker).await;
    }

    let client = req
        .extensions()
        .get::<Client<HttpsConnector<HttpConnector>, Incoming>>()
        .cloned()
        .unwrap()
        .clone();

    let (parts, body) = req.into_parts();

    // Capture the original host and scheme for redirect rewriting
    let original_host = parts
        .headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap(); // Convert to &str safely
    //println!("original_host: {}", original_host);

    // Prepare antibot
    let is_antibot_protected = is_domain_configured_for_antibot(
        frontend_name.clone(),
        original_host.clone(),
        config.clone(),
    );

    // upstream uri
    let mut upstream_uri = get_upstream_uri(original_host.clone(), servers_tracker.clone(), false);
    if upstream_uri == "" {
        // Internal server - No server available
        upstream_uri = format!(
            "http://127.0.0.1:{}/{}{}",
            HTTP_INTERNAL_SERVER,
            INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,
            parts.uri.to_string()
        );
    } else {
        // antibot for this host ?
        if is_antibot_protected {
            if !is_cookie_antibot(parts.headers.get("cookie")) {
                upstream_uri = format!(
                    "http://127.0.0.1:{}/{}",
                    HTTP_INTERNAL_SERVER, INTERNAL_ROUTE_ANTIBOT,
                );
            }
        }
        upstream_uri = format!("{}{}", upstream_uri, parts.uri.to_string());
    }
    let upstream_uri = upstream_uri.parse::<Uri>().unwrap();
    //====> To check round robin load balance
    // println!("upstream_uri: {}", upstream_uri);

    // Build forwarded request with all original headers
    let forwarded_req = {
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
    };

    // println!("Forwarding traffic for {}", name);
    let response = client.request(forwarded_req).await;

    match response {
        Ok(mut response) => {
            let original_host = original_host.clone();
            set_response_header(original_host, &mut response).await;
            //println!("Response before sending to http server: {:?}", response);
            Ok::<Response<body::Incoming>, hyper_util::client::legacy::Error>(response)
        }
        Err(e) => {
            eprintln!("Request forwarding error: {:?}", e,);
            // @todo
            // set backend disabled
            // return html content
            Err(e)
        }
    }
}
