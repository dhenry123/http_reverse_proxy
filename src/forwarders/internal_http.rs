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
        INTERNAL_ROUTE_ANTIBOT, INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,
        INTERNAL_ROUTE_MAKE_WEBSOCKET,
    },
    html::{template_html_antibot, template_html_internal_error},
    structs::GenericError,
};

use super::forwarder_helper::get_cookie_antibot;

enum InternalServerErrors {
    ServerUnavailable,
    RouteNotFound,
}

async fn internal_error(
    error: InternalServerErrors,
    parts: http::request::Parts,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let error_code: String;
    let p1: String;
    let p2: String;
    match error {
        InternalServerErrors::RouteNotFound => {
            error_code = "Fallback Route not found".to_string();
            p1 = "???".to_string();
            p2 = "???".to_string();
        }
        InternalServerErrors::ServerUnavailable => {
            error_code = "503 Service Unavailable".to_string();
            p1 = "Our servers are temporarily unavailable due to high traffic or maintenance. Please try again later.".to_string();
            p2 = "If the problem persists, contact <a href='mailto:support@example.com'>support@example.com</a>.".to_string();
        }
    }
    let final_path = parts.uri.to_string().replace(
        format!("/{}", INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE).as_str(),
        "",
    );
    let html = template_html_internal_error(error_code, p1, p2, final_path);
    let body = Full::new(Bytes::from(html));
    //println!("body: {:?}", body);
    let mut response = Response::new(body);
    // Change http code
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    Ok(response)
}

async fn antibot(parts: http::request::Parts) -> Result<Response<Full<Bytes>>, Infallible> {
    // Remove /ANTIBOT_INTERNAL_ROUTE
    let final_path = parts
        .uri
        .to_string()
        .replace(format!("/{}", INTERNAL_ROUTE_ANTIBOT).as_str(), "");
    //println!("finale path: {}", final_path);
    let html = template_html_antibot(final_path);
    let body = Full::new(Bytes::from(html));
    let mut response = Response::new(body);
    // Change http code
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;

    let host = parts.headers.get("host");
    if host.is_some() {
        let cookie = get_cookie_antibot(host.unwrap().to_str().unwrap().to_string());
        response.headers_mut().append(
            "Set-Cookie",
            HeaderValue::from_str(cookie.to_string().as_str()).unwrap(),
        );
    }
    Ok(response)
}

pub async fn ws_upgrade_reponse(accept: String) -> Result<Response<Full<Bytes>>, Infallible> {
    let body = Full::new(Bytes::from("".to_string()));
    let mut response = Response::new(body);
    // Change http code
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    // Upgrade
    response
        .headers_mut()
        .append("Upgrade", HeaderValue::from_str("websocket").unwrap());
    response
        .headers_mut()
        .append("Connection", HeaderValue::from_str("upgrade").unwrap());
    response.headers_mut().append(
        "Sec-WebSocket-Accept",
        HeaderValue::from_str(accept.as_str()).unwrap(),
    );

    Ok(response)
}

async fn backend_service(
    req: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (parts, _body) = req.into_parts();
    //println!("route : {:?}", parts.uri);
    match (parts.clone().method, parts.uri.path()) {
        // Server unavailable
        (Method::GET, path)
            if path.starts_with(
                format!("/{}", INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE,).as_str(),
            ) =>
        {
            Ok(internal_error(InternalServerErrors::ServerUnavailable, parts).await?)
        }
        // Antibot
        (Method::GET, path)
            if path.starts_with(format!("/{}", INTERNAL_ROUTE_ANTIBOT,).as_str()) =>
        {
            Ok(antibot(parts).await?)
        }
        // Create web socket Response
        (Method::GET, path)
            if path.starts_with(format!("/{}", INTERNAL_ROUTE_MAKE_WEBSOCKET,).as_str()) =>
        {
            let token = path.replace(format!("/{}/", INTERNAL_ROUTE_MAKE_WEBSOCKET).as_str(), "");
            Ok(ws_upgrade_reponse(token).await?)
        }
        // else
        _ => Ok(internal_error(InternalServerErrors::RouteNotFound, parts).await?),
    }
}

pub async fn internal_http(name: String, addr: SocketAddr) -> Result<(), GenericError> {
    println!("Internal HTTP listener: {} is listening on: {}", name, addr);

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
