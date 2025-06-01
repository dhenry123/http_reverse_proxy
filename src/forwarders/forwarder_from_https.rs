use hyper::{Request, server::conn::http1, service::service_fn};

use hyper_util::rt::{TokioIo, TokioTimer};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::RwLock};
use tokio_rustls::TlsAcceptor;

use crate::{
    config_manager::ConfigManager,
    forwarders::{forwarder_handler::handle_request, forwarder_helper::get_http_client},
    structs::GenericError,
};

pub async fn proxy_from_https(
    config_manager: Arc<RwLock<ConfigManager>>,
    frontend_name: String,
    addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
) -> Result<(), GenericError> {
    let client = get_http_client();

    // Listener
    let listener = TcpListener::bind(addr).await?;
    println!(
        "HTTPS listener: {} is listening on: {}",
        frontend_name.clone(),
        addr
    );

    // Use a connection pool or limit for production
    let connection_limit = tokio::sync::Semaphore::new(100); // Adjust based on expected load

    loop {
        // Acquire permit before accepting connection
        let permit = connection_limit
            .acquire()
            .await
            .map_err(|_| GenericError::from("Connection limit reached"))?;

        match listener.accept().await {
            Ok((tcp, peer_addr)) => {
                // println!("_peer_addr: {:?}", peer_addr);
                let _permit = permit;
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
                    // Create the service_fn
                    service_fn(move |req: Request<hyper::body::Incoming>| {
                        // Call the handler - no async/await here!
                        handle_request(
                            req,
                            peer_addr,
                            frontend_name.clone(),
                            servers_tracker.clone(),
                            config_manager.clone(),
                            client.clone(),
                        )
                    })
                };
                let tls_acceptor = tls_acceptor.clone();
                // connection accepted - let's check tls and continue if ok
                let frontend_name = frontend_name.clone();
                match tls_acceptor.accept(tcp).await {
                    Ok(tls_stream) => {
                        // Handle the connection
                        let io = TokioIo::new(tls_stream);
                        let svc = svc.clone();

                        tokio::task::spawn(async move {
                            if let Err(err) = http1::Builder::new()
                                .timer(TokioTimer::new())
                                .header_read_timeout(Some(Duration::from_secs(5)))
                                .auto_date_header(false)
                                .serve_connection(io, svc)
                                .with_upgrades()
                                .await
                            {
                                if !err.is_timeout() {
                                    eprintln!(
                                        "[https listener error]: name: {} - from: {} - error: {:?}",
                                        frontend_name, peer_addr, err
                                    )
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("TLS failed: {} - peer: {}", e, peer_addr);
                        if let Some(inner) = e.get_ref() {
                            eprintln!("Root cause: {:?}", inner.source());
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    eprintln!("[ACCEPT ERROR] {:?}", e);
                }
            }
        }
    }
}
