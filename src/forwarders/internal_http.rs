use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode, server::conn::http1, service::service_fn};

use hyper_util::rt::{TokioIo, TokioTimer};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;

use crate::structs::GenericError;

fn get_service_unavailable() -> &'static str {
    r#"
    <!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Service Unavailable</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #f8f9fa;
            color: #343a40;
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            text-align: center;
        }

        .error-container {
            max-width: 500px;
            padding: 2rem;
            background: white;
            border-radius: 10px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        h1 {
            font-size: 3rem;
            margin: 0;
            color: #dc3545;
        }

        .error-code {
            font-size: 1.2rem;
            font-weight: bold;
            margin: 0.5rem 0;
        }

        p {
            margin: 1rem 0;
            line-height: 1.5;
        }

        a {
            color: #007bff;
            text-decoration: none;
        }

        a:hover {
            text-decoration: underline;
        }

        .btn {
            display: inline-block;
            margin-top: 1rem;
            padding: 0.5rem 1rem;
            background: #007bff;
            color: white;
            border-radius: 5px;
            transition: background 0.3s;
        }

        .btn:hover {
            background: #0056b3;
            text-decoration: none;
        }
    </style>
</head>
<body>
    <div class="error-container">
        <h1>Oops!</h1>
        <div class="error-code">503 Service Unavailable</div>
        <p>Our servers are temporarily unavailable due to high traffic or maintenance. Please try again later.</p>
        <p>If the problem persists, contact <a href="mailto:support@example.com">support@example.com</a>.</p>
        <a href="/" class="btn">Refresh Page</a>
    </div>
</body>
</html>
"#
}

async fn backend_service_disabled(
    _: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let html = get_service_unavailable();
    let body = Full::new(Bytes::from(html));
    //println!("body: {:?}", body);
    let mut response = Response::new(body);
    //change http code
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    Ok(response)
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
                        .serve_connection(io, service_fn(backend_service_disabled))
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
