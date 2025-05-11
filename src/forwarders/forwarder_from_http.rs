use arc_swap::ArcSwap;
use hyper::{Request, server::conn::http1, service::service_fn};

use hyper_util::rt::{TokioIo, TokioTimer};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

use crate::{
    forwarders::{forwarder_handler::handle_request, forwarder_helper::get_http_client},
    structs::{GenericError, ProxyConfig},
};

use super::servers_tracker::ServerTracker;

pub async fn proxy_from_http(
    config: Arc<ArcSwap<ProxyConfig>>,
    servers_tracker: Arc<arc_swap::ArcSwapAny<Arc<ServerTracker>>>,
    frontend_name: String,
    addr: SocketAddr,
) -> Result<(), GenericError> {
    println!("HTTP listener: {} is listening on: {}", frontend_name, addr);

    let client = get_http_client();
    let listener = TcpListener::bind(addr).await?;

    loop {
        match listener.accept().await {
            Ok((tcp, peer_addr)) => {
                let svc = {
                    // Clone the values we need to move into the closure
                    let client = client.clone();
                    let servers_tracker = servers_tracker.clone();
                    let config = config.clone();
                    let frontend_name = frontend_name.clone();

                    // Create the service_fn
                    service_fn(move |mut req: Request<hyper::body::Incoming>| {
                        // Insert extensions
                        req.extensions_mut().insert(frontend_name.clone());
                        req.extensions_mut().insert(config.clone());
                        req.extensions_mut().insert(peer_addr);
                        req.extensions_mut().insert(client.clone());
                        req.extensions_mut().insert(servers_tracker.clone());

                        // Call the handler - no async/await here!
                        handle_request(req)
                    })
                };
                let io = TokioIo::new(tcp);

                tokio::task::spawn(async move {
                    let svc = svc.clone();
                    if let Err(err) = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .preserve_header_case(true)
                        .writev(true)
                        .serve_connection(io, svc)
                        .with_upgrades()
                        .await
                    {
                        eprintln!("[ERROR] Error serving connection: {:?}", err);
                    }
                });
            }
            Err(e) => {
                eprintln!("[ACCEPT ERROR] {:?}", e);
            }
        }
    }
}
