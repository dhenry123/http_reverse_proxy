use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use log::{debug, info};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

use crate::{
    api::{
        embed_react::serve_embedded_file, list::api_list_config_object,
        metrics::api_metric_get_hits, server::api_server_active_set,
    },
    config_manager::ConfigManager,
    constants::{
        API_BACKENDS_LIST, API_FRONTENDS_LIST, API_METRICS_GET_HITS, API_SERVERS_ACTIVE,
        API_SERVERS_LIST, API_VERSION,
    },
    forwarders::internal_http::{InternalServerErrors, internal_error},
    state::AppState,
    structs::{ApiOjectTypes, GenericError},
};

async fn backend_service(
    req: Request<hyper::body::Incoming>,
    config_manager: Arc<RwLock<ConfigManager>>,
    peer_addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
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
        //Enbed react app
        (Method::GET, path) if !path.starts_with(format!("/{}", API_VERSION).as_str()) => {
            Ok(serve_embedded_file(path).await?)
        }
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
        // ---> set server active attribute
        (Method::PUT, path)
            if path.starts_with(format!("/{}/{}", API_VERSION, API_SERVERS_ACTIVE,).as_str()) =>
        {
            Ok(api_server_active_set(config_manager.clone(), body_bytes).await?)
        }
        // metrics
        // ----> hits
        (Method::GET, path)
            if path.starts_with(format!("/{}/{}", API_VERSION, API_METRICS_GET_HITS,).as_str()) =>
        {
            Ok(api_metric_get_hits(state, &parts)?)
        }
        // else
        _ => Ok(internal_error(InternalServerErrors::RouteNotFound, parts).await?),
    }
}

pub async fn apirest_http(
    config_manager: Arc<tokio::sync::RwLock<ConfigManager>>,
    frontend_name: String,
    addr: SocketAddr,
    state: Arc<AppState>,
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
                let state = state.clone();
                let svc = {
                    // Clone again for the service_fn
                    let config_manager = Arc::clone(&config_manager);
                    let state = state.clone();
                    // Create the service_fn
                    service_fn(move |req: Request<hyper::body::Incoming>| {
                        backend_service(req, config_manager.clone(), peer_addr, state.clone())
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
