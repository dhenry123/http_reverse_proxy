use hyper::{Request, server::conn::http1, service::service_fn};

use hyper_util::rt::{TokioIo, TokioTimer};
use log::info;
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

use crate::{
    config_manager::ConfigManager,
    forwarders::{forwarder_handler::handle_request, forwarder_helper::get_http_client},
    state::AppState,
    structs::GenericError,
};

pub async fn proxy_from_http(
    config_manager: Arc<RwLock<ConfigManager>>,
    frontend_name: String,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), GenericError> {
    info!(
        "HTTP listener: {} is listening on: {}",
        &frontend_name, addr
    );

    let client = get_http_client();
    let listener = TcpListener::bind(addr).await?;

    loop {
        match listener.accept().await {
            Ok((tcp, peer_addr)) => {
                let frontend_name = frontend_name.clone();
                let state = state.clone();
                let svc = {
                    // Clone the values we need to move into the closure
                    let client = client.clone();
                    let servers_tracker = config_manager
                        .read()
                        .await
                        .get_tracker(frontend_name.clone())
                        .ok_or(GenericError::from("No servers available"))?;

                    let config_manager = config_manager.clone();
                    let frontend_name = frontend_name.clone();
                    let state = state.clone();

                    // Create the service_fn
                    service_fn(move |req: Request<hyper::body::Incoming>| {
                        state.metrics.increment_frontend(&frontend_name);
                        // Call the handler - no async/await here!
                        handle_request(
                            req,
                            peer_addr,
                            frontend_name.clone(),
                            servers_tracker.clone(),
                            config_manager.clone(),
                            client.clone(),
                            state.clone(),
                        )
                    })
                };
                let io = TokioIo::new(tcp);

                tokio::task::spawn(async move {
                    let svc = svc.clone();
                    let frontend_name = frontend_name.clone();
                    if let Err(err) = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .preserve_header_case(true)
                        .writev(true)
                        .serve_connection(io, svc)
                        .with_upgrades()
                        .await
                    {
                        // no display if IncompleteMessage
                        if !err.is_incomplete_message() {
                            log::error!(
                                "[https listener error]: name: {} - from: {} - error: {:?}",
                                frontend_name,
                                peer_addr,
                                err
                            );
                        }
                    }
                });
            }
            Err(e) => {
                log::error!("[ACCEPT ERROR] {:?}", e);
            }
        }
    }
}
