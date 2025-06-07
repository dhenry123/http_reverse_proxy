use futures::Sink;
use futures_util::{SinkExt, stream::StreamExt};
use hyper::{Method, Request, Response, Uri, body};
use hyper_util::rt::TokioIo;
use log::debug;
use std::sync::Arc;

use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, Message},
};

use crate::{
    constants::{INTERNAL_ROUTE_MAKE_WEBSOCKET, SECRET_WS_GUID},
    forwarders::forwarder_helper::build_upstream_uri,
    internal_server_free_port,
};

use super::{
    backend::Backend,
    forwarder_helper::{get_http_client, get_upstream_server},
};

pub async fn handle_websocket_upgrade(
    mut req: Request<hyper::body::Incoming>,
    servers_tracker: &Arc<Backend>,
) -> Result<Response<body::Incoming>, hyper_util::client::legacy::Error> {
    let upgraded_fut = hyper::upgrade::on(&mut req);

    let (parts, body) = req.into_parts();
    let original_host = parts
        .headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap(); // Convert to &str safely

    let upstream_server = get_upstream_server(original_host.clone(), &servers_tracker);
    let upstream_uri = match upstream_server {
        Some(server) => build_upstream_uri(server, true),
        None => "".to_string(),
    };
    let upstream_uri = upstream_uri.parse::<Uri>().unwrap();

    debug!("upstream_uri: {}", upstream_uri);

    // Generate WebSocket accept key
    let key = parts
        .headers
        .get("Sec-WebSocket-Key")
        .ok_or("Missing Sec-WebSocket-Key header");

    if key.is_err() {
        log::error!("Error on websocket key Result: {:?}", key);
    }
    let accept = generate_accept_key(key.unwrap().as_bytes());

    // Create the upgrade response from internal (because of the Incoming type)
    let internal_upstream_uri = format!(
        "http://127.0.0.1:{}/{}/{}",
        internal_server_free_port::get_global_port(),
        INTERNAL_ROUTE_MAKE_WEBSOCKET,
        accept
    );
    debug!("Uri to get websocket header: {}", internal_upstream_uri);
    let forwarded_req = {
        let builder = Request::builder()
            .method(Method::GET)
            .uri(internal_upstream_uri);
        builder.body(body).unwrap()
    };

    let client = get_http_client();
    let response = client.request(forwarded_req).await;

    // Spawn a task to handle the WebSocket connection
    tokio::spawn(async move {
        let upgraded = upgraded_fut
            .await
            .expect("Error during WebSocket handshake");
        let tokio_io = TokioIo::new(upgraded);

        // Don't call accept_async on the already upgraded connection
        let ws_server = tokio_tungstenite::WebSocketStream::from_raw_socket(
            tokio_io,
            tokio_tungstenite::tungstenite::protocol::Role::Server,
            None,
        )
        .await;

        // Connection to backend
        let (ws_upstream, _) = connect_async(upstream_uri)
            .await
            .expect("Failed to connect to destination");

        // Linking streams
        let (ws_server_sender, ws_server_receiver) = ws_server.split();
        let (ws_upstream_sender, ws_upstream_receiver) = ws_upstream.split();

        let server_to_upstream =
            forward_messages(ws_server_receiver, ws_upstream_sender, "upstream");
        let upstream_to_server = forward_messages(ws_upstream_receiver, ws_server_sender, "client");

        tokio::select! {
            _ = server_to_upstream => {
                debug!("Client→upstream task completed");
            },
            _ = upstream_to_server => {
                debug!("Upstream→client task completed");
            },
        }
    });

    Ok::<Response<body::Incoming>, hyper_util::client::legacy::Error>(response.unwrap())
}

/**
 * based on SECRET_WS_GUID
 */
fn generate_accept_key(key: &[u8]) -> String {
    use base64::Engine;
    use sha1::{Digest, Sha1};
    let mut sha1 = Sha1::new();
    sha1.update(key);
    sha1.update(SECRET_WS_GUID.as_bytes());
    let hash = sha1.finalize();
    base64::engine::general_purpose::STANDARD.encode(hash)
}

/**
 * Forwarding logic
 */
async fn forward_messages<SRC, DST, E>(mut source: SRC, mut dest: DST, direction: &str)
where
    SRC: StreamExt<Item = Result<Message, tungstenite::Error>> + Unpin,
    DST: Sink<Message, Error = E> + Unpin,
    E: std::fmt::Display, // Required for error formatting
{
    while let Some(msg) = source.next().await {
        match msg {
            Ok(Message::Pong(_)) => continue, // Ignore pong replies
            Ok(msg) => {
                if msg.is_close() {
                    debug!("{} closed the connection", direction);
                    let _ = dest.close().await;
                    break;
                }
                debug!("Sending message: {}", msg);
                if let Err(e) = dest.send(msg).await {
                    log::error!("Failed sending to {}: {}", direction, e);
                    break;
                }
            }
            Err(e) => {
                log::error!("{} message error: {}", direction, e);
                break;
            }
        }
    }
}
