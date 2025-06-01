use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use log::{debug, info};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

use crate::{
    api::{list::api_list_config_object, server::api_server_active_set},
    config_manager::ConfigManager,
    constants::{
        API_BACKENDS_LIST, API_FRONTENDS_LIST, API_SERVERS_ACTIVE, API_SERVERS_LIST, API_VERSION,
    },
    forwarders::internal_http::{InternalServerErrors, internal_error},
    structs::{ApiOjectTypes, GenericError},
};

async fn backend_service(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    //let peer_addr = req.extensions().get::<SocketAddr>().cloned().unwrap();
    let config_manager = req
        .extensions()
        .get::<Arc<RwLock<ConfigManager>>>()
        .cloned()
        .unwrap()
        .clone();

    let proxy_config = config_manager.read().await.get_config().await;
    let (parts, body) = req.into_parts();

    //Collect body if provided
    let body_bytes: Option<Bytes> = match body.boxed().collect().await {
        Ok(collected_body) => Some(collected_body.to_bytes()),
        Err(_) => None,
    };

    debug!("parts: {:?}", parts);
    debug!("query: {:?}", parts.uri.query());
    debug!("route : {:?}", parts.uri);
    match (parts.clone().method, parts.uri.path()) {
        // List
        // ---> frontends
        (Method::GET, path)
            if path.starts_with(format!("/{}/{}", API_VERSION, API_FRONTENDS_LIST,).as_str()) =>
        {
            Ok(api_list_config_object(ApiOjectTypes::Frontends, proxy_config).await?)
        }
        // ---> backends
        (Method::GET, path)
            if path.starts_with(format!("/{}/{}", API_VERSION, API_BACKENDS_LIST,).as_str()) =>
        {
            Ok(api_list_config_object(ApiOjectTypes::PoolBackends, proxy_config).await?)
        }
        // ---> servers
        (Method::GET, path)
            if path.starts_with(format!("/{}/{}", API_VERSION, API_SERVERS_LIST,).as_str()) =>
        {
            Ok(api_list_config_object(ApiOjectTypes::PoolServers, proxy_config).await?)
        }
        // ---> test
        (Method::PUT, path)
            if path.starts_with(format!("/{}/{}", API_VERSION, API_SERVERS_ACTIVE,).as_str()) =>
        {
            Ok(api_server_active_set(config_manager.clone(), body_bytes).await?)
        }
        // else
        _ => Ok(internal_error(InternalServerErrors::RouteNotFound, parts).await?),
    }
}

pub async fn apirest_http(
    config_manager: Arc<tokio::sync::RwLock<ConfigManager>>,
    frontend_name: String,
    addr: SocketAddr,
) -> Result<(), GenericError> {
    info!(
        "API REST HTTP listener: {} is listening on: {}",
        frontend_name, addr
    );

    let listener = TcpListener::bind(addr).await?;

    loop {
        match listener.accept().await {
            Ok((tcp, peer_addr)) => {
                let config_manager = config_manager.clone();
                let svc = {
                    // Clone again for the service_fn
                    let config_manager = Arc::clone(&config_manager);
                    // Create the service_fn
                    service_fn(move |mut req: Request<hyper::body::Incoming>| {
                        // Insert extensions
                        req.extensions_mut().insert(config_manager.clone());
                        req.extensions_mut().insert(peer_addr);
                        // Call the handler - no async/await here!
                        backend_service(req)
                    })
                };
                let io = TokioIo::new(tcp);
                tokio::task::spawn(async move {
                    let svc = svc.clone();
                    if let Err(err) = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .keep_alive(true)
                        .preserve_header_case(true)
                        .writev(true)
                        .serve_connection(io, svc)
                        .await
                    {
                        log::error!("[internal listener error] {:?}", err);
                    }
                });
            }
            Err(e) => {
                // Only log persistent errors
                log::error!("[internal listener ACCEPT ERROR] {:?}", e);
            }
        }
    }
}
