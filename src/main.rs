mod config_manager;
mod constants;
mod forwarders;
mod html;
mod internal_server_free_port;
mod structs;

use arc_swap::ArcSwap;
use clap::Parser;
use config_manager::{Args, ConfigManager};
use forwarders::forwarder_from_http::proxy_from_http;
use forwarders::forwarder_from_https::proxy_from_https;
use forwarders::internal_http::internal_http;
use forwarders::servers_tracker::ServerTracker;
use std::process;
use structs::GenericError;

use std::net::{IpAddr, SocketAddr};

use std::sync::Arc;

fn parse_bind_address(input: &str) -> Result<IpAddr, String> {
    input
        .parse()
        .map_err(|e| format!("Invalid IP address '{}': {}", input, e))
}

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    let args = Args::parse();
    let mut config_manager = ConfigManager::new(args);
    config_manager.load().await?;
    let config = config_manager.get_config().await;
    let certs_path = config_manager.get_config_tls_certs_path().await;
    let mut listeners = Vec::new();
    // Starting frontends
    for frontend in config.load().as_ref().clone().frontends {
        if !frontend.active {
            continue;
        }
        // Clone init configuratin
        let ipaddr = parse_bind_address(&frontend.addr).unwrap();
        let addr = SocketAddr::from((ipaddr, frontend.port));
        let cfg = config.clone();
        let server_task: tokio::task::JoinHandle<()>;
        let certs_path = certs_path.clone();
        if frontend.tls {
            // Frontend https
            server_task = tokio::spawn(async move {
                let servers_tracker: Arc<arc_swap::ArcSwapAny<Arc<ServerTracker>>> =
                    Arc::new(ArcSwap::new({
                        let mut tracker = ServerTracker::new();
                        tracker.populate(frontend.clone().name, cfg.clone());
                        Arc::new(tracker)
                    }));
                let certs_path = certs_path.clone();

                if let Err(e) = proxy_from_https(
                    cfg.clone(),
                    certs_path,
                    servers_tracker,
                    frontend.clone().name,
                    addr,
                )
                .await
                {
                    eprintln!("Frontend {} crashed: {}", frontend.name, e);
                }
            });
        } else {
            // Frontend http
            server_task = tokio::spawn(async move {
                let servers_tracker: Arc<arc_swap::ArcSwapAny<Arc<ServerTracker>>> =
                    Arc::new(ArcSwap::new({
                        let mut tracker = ServerTracker::new();
                        tracker.populate(frontend.clone().name, cfg.clone());
                        Arc::new(tracker)
                    }));

                if let Err(e) =
                    proxy_from_http(cfg.clone(), servers_tracker, frontend.clone().name, addr).await
                {
                    eprintln!("Frontend {} crashed: {}", frontend.name, e);
                }
            });
        }
        listeners.push(server_task);
    }
    // Internal frontend http (hard because i don't know how to implement a fake Response<Incoming> in listeners when backend is disabled
    let ipaddr = parse_bind_address("127.0.0.1").unwrap();
    let port = internal_server_free_port::init_global_port(23000, 27000);
    let addr = SocketAddr::from((ipaddr, port));
    let server_task: tokio::task::JoinHandle<()>;

    let frontend_name = "internal".to_string();
    server_task = tokio::spawn(async move {
        if let Err(e) = internal_http(frontend_name.clone(), addr).await {
            eprintln!("Frontend {} crashed: {}", frontend_name, e);
            eprintln!("Fatal error, exiting");
            process::exit(10);
        }
    });
    listeners.push(server_task);

    // Wait for CTRL+C or all servers to exit
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Shutdown signal received");
        }
        _ = async {
            for server in listeners {
                let _ = server.await;
            }
        } => {
            println!("All frontend servers terminated");
        }
    }
    Ok(())
}
