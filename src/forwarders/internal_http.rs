use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    Request, Response, StatusCode, header::HeaderValue, server::conn::http1, service::service_fn,
};

use hyper_util::rt::{TokioIo, TokioTimer};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::http;
use uuid::Uuid;

use crate::{
    constants::{
        ANTIBOT_COOKIE_NAME, ANTIBOT_INTERNAL_ROUTE,
        INTERNAL_ERROR_ROUTE_NO_BACKEND_SERVER_AVAILABLE,
    },
    html::{template_html_antibot, template_html_internal_error},
    structs::GenericError,
};

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
        _ => {
            error_code = "".to_string();
            p1 = "".to_string();
            p2 = "".to_string();
        }
    }
    let final_path = parts.uri.to_string().replace(
        format!("/{}", INTERNAL_ERROR_ROUTE_NO_BACKEND_SERVER_AVAILABLE).as_str(),
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
        .replace(format!("/{}", ANTIBOT_INTERNAL_ROUTE).as_str(), "");
    //println!("finale path: {}", final_path);
    let html = template_html_antibot(final_path);
    let body = Full::new(Bytes::from(html));
    let mut response = Response::new(body);
    // Change http code
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    // Set Antibot cookie (basic To improve @todo)
    response.headers_mut().append(
        "Set-Cookie",
        HeaderValue::from_str(
            format!("{}={}; Path=/", ANTIBOT_COOKIE_NAME, Uuid::new_v4()).as_str(),
        )
        .unwrap(),
    );
    Ok(response)
}

async fn backend_service(
    req: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (parts, _body) = req.into_parts();
    //println!("route : {:?}", parts.uri);
    if parts
        .uri
        .to_string()
        .matches(format!("/{}", INTERNAL_ERROR_ROUTE_NO_BACKEND_SERVER_AVAILABLE,).as_str())
        .count()
        > 0
    {
        Ok(internal_error(InternalServerErrors::ServerUnavailable, parts).await?)
    } else if parts
        .uri
        .to_string()
        .matches(format!("/{}", ANTIBOT_INTERNAL_ROUTE).as_str())
        .count()
        > 0
    {
        //println!("[*] match antibot");
        Ok(antibot(parts).await?)
    } else {
        Ok(internal_error(InternalServerErrors::RouteNotFound, parts).await?)
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
                        eprintln!("[ERROR] Error serving connection: {:?}", err);
                    }
                });
            }
            Err(e) => {
                // Only log persistent errors
                eprintln!("[ACCEPT ERROR] {:?}", e);
            }
        }
    }
}
