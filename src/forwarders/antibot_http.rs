use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    Request, Response, StatusCode, header::HeaderValue, server::conn::http1, service::service_fn,
};

use crate::{constants::ANTIBOT_COOKIE_NAME, structs::GenericError};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use uuid::Uuid;

fn get_service_unavailable(path: String) -> String {
    static HTML_TEMPLATE: &str = r#"
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
        <h1>ANTIBOT process!</h1>
        <p></p>
        <p>Clic on refresh</p>
        <a href="PATHREFRESH" class="btn">Refresh Page</a>
    </div>
</body>
</html>
"#;
    let html = String::from(HTML_TEMPLATE).clone();
    html.replace("PATHREFRESH", &path.as_str())
}

async fn backend_home_antibot(
    req: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let (parts, _body) = req.into_parts();
    let path = parts
        .uri
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let html = get_service_unavailable(path);
    let body = Full::new(Bytes::from(html));
    //println!("body: {:?}", body);
    let mut response = Response::new(body);
    //change http code
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response.headers_mut().append(
        "Set-Cookie",
        HeaderValue::from_str(
            format!("{}={}; Path=/", ANTIBOT_COOKIE_NAME, Uuid::new_v4()).as_str(),
        )
        .unwrap(),
    );
    Ok(response)
}

pub async fn antibot_http(name: String, addr: SocketAddr) -> Result<(), GenericError> {
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
                        .serve_connection(io, service_fn(backend_home_antibot))
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
