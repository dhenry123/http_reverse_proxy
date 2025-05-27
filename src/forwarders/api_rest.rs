use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode, header::HeaderValue, server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::http;

use crate::{
    constants::{
        API_HOSTS_LIST, INTERNAL_ROUTE_ANTIBOT, INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,
        INTERNAL_ROUTE_MAKE_WEBSOCKET,
    },
    structs::GenericError,
};

enum InternalServerErrors {
    ServerUnavailable,
    RouteNotFound,
}

async fn hosts_list(parts: http::request::Parts) -> Result<Response<Full<Bytes>>, Infallible> {
    let html = "response hosts list";
    let body = Full::new(Bytes::from(html));
    //println!("body: {:?}", body);
    let mut response = Response::new(body);
    // Change http code
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    Ok(response)
}

async fn backend_service(
    req: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (parts, _body) = req.into_parts();
    //println!("route : {:?}", parts.uri);
    match (parts.clone().method, parts.uri.path()) {
        // Server unavailable
        (Method::GET, path) if path.starts_with(format!("/{}", API_HOSTS_LIST,).as_str()) => {
            Ok(hosts_list(parts).await?)
        }
        // else
        _ => Ok(super::internal_http::internal_error(
            super::internal_http::InternalServerErrors::RouteNotFound,
            parts,
        )
        .await?),
    }
}

pub async fn apirest_http(name: String, addr: SocketAddr) -> Result<(), GenericError> {
    println!("API REST HTTP listener: {} is listening on: {}", name, addr);

    let listener = TcpListener::bind(addr).await?;

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let io = TokioIo::new(tcp);

                tokio::task::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .keep_alive(true)
                        .preserve_header_case(true)
                        .writev(true)
                        .serve_connection(io, service_fn(backend_service))
                        .await
                    {
                        eprintln!("[internal listener error] {:?}", err);
                    }
                });
            }
            Err(e) => {
                // Only log persistent errors
                eprintln!("[internal listener ACCEPT ERROR] {:?}", e);
            }
        }
    }
}
