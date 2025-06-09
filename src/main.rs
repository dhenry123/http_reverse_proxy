mod api;
mod config_manager;
mod constants;
mod forwarders;
mod html;
mod http_response;
mod init;
mod internal_server_free_port;
mod state;
mod statistics;
mod structs;

use api::api_rest::apirest_http;
use clap::Parser;
use config_manager::{Args, ConfigManager};
use constants::{API_LISTENING_ADDR, API_LISTENING_PORT};
use forwarders::forwarder_from_http::proxy_from_http;
use forwarders::forwarder_from_https::proxy_from_https;
use forwarders::internal_http::internal_http;
use forwarders::tls_acceptor::tls_acceptor_init;
use init::init_logging;
use log::info;
use state::AppState;
use std::{env, process, sync::Arc};
use structs::GenericError;
use tokio::sync::RwLock;

use std::net::{IpAddr, SocketAddr};

fn parse_bind_address(input: &str) -> Result<IpAddr, String> {
    input
        .parse()
        .map_err(|e| format!("Invalid IP address '{}': {}", input, e))
}

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    // initializing log
    init_logging();
    // load configuration from yaml - mutable because expected configuration could be modified via API
    let args = Args::parse();
    let mut config_manager = ConfigManager::new(args);
    config_manager.load().await?;

    // One TLS Acceptor
    let certs_path = config_manager.get_config_tls_certs_path().await;
    let tls_acceptor = tls_acceptor_init(certs_path)?;

    // config object
    let config = config_manager.get_config().await;

    // config manager must be mutable in this process
    let config_manager_shared = Arc::new(RwLock::new(config_manager));

    let state = AppState::new(config_manager_shared.clone());

    // Starting frontends
    let mut listeners = Vec::new();
    for frontend in config.frontends.clone() {
        let shared_manager = config_manager_shared.clone();
        if !frontend.enabled {
            continue;
        }
        let ipaddr = parse_bind_address(&frontend.addr).unwrap();
        let frontend_addr = SocketAddr::from((ipaddr, frontend.port));
        let listener: tokio::task::JoinHandle<()>;
        let state = state.clone();

        if frontend.tls {
            // Frontend https
            let tls_acceptor = tls_acceptor.clone();
            listener = tokio::spawn(async move {
                if let Err(e) = proxy_from_https(
                    shared_manager,
                    frontend.clone().name,
                    frontend_addr,
                    state,
                    tls_acceptor,
                )
                .await
                {
                    log::error!("[Error] Frontend {} crashed: {}", frontend.name, e);
                }
            });
        } else {
            // Frontend http
            listener = tokio::spawn(async move {
                if let Err(e) =
                    proxy_from_http(shared_manager, frontend.clone().name, frontend_addr, state)
                        .await
                {
                    log::error!("[Error] Frontend {} crashed: {}", frontend.name, e);
                }
            });
        }
        listeners.push(listener);
    }

    // Internal frontend http (hard because i don't know how to implement a fake Response<Incoming> in listeners when backend is disabled
    let ipaddr = parse_bind_address("127.0.0.1").unwrap();
    let port = internal_server_free_port::init_global_port(23000, 27000);
    let frontend_addr = SocketAddr::from((ipaddr, port));
    let listener: tokio::task::JoinHandle<()>;

    let frontend_name = "internal".to_string();
    listener = tokio::spawn(async move {
        if let Err(e) = internal_http(frontend_name.clone(), frontend_addr).await {
            log::error!("[Error] Frontend {} crashed: {}", frontend_name, e);
            log::error!("Fatal error, exiting");
            process::exit(10);
        }
    });

    listeners.push(listener);

    // API Rest server
    let ipaddr = parse_bind_address(
        env::var("API_LISTENING_ADDR")
            .as_deref()
            .unwrap_or(API_LISTENING_ADDR),
    )
    .unwrap();
    let frontend_addr = SocketAddr::from((ipaddr, API_LISTENING_PORT));
    let listener: tokio::task::JoinHandle<()>;
    let frontend_name = "APIRest".to_string();
    listener = tokio::spawn(async move {
        let shared_manager = config_manager_shared.clone();
        if let Err(e) = apirest_http(
            shared_manager.clone(),
            frontend_name.clone(),
            frontend_addr,
            state.clone(),
        )
        .await
        {
            log::error!("[Error] Api rest {} crashed: {}", frontend_name, e);
            log::error!("Fatal error, exiting");
            process::exit(10);
        }
    });
    listeners.push(listener);

    // Wait for CTRL+C or all servers to exit
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
        _ = async {
            for listener in listeners {
                let _ = listener.await;
            }
        } => {
            info!("All frontend servers terminated");
        }
    }
    Ok(())
}
