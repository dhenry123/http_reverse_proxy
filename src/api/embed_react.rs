use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./src/api/react_dist"]
struct Asset;

pub async fn serve_embedded_file(path: &str) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = if path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    if let Some(file) = Asset::get(path) {
        let mime_type = from_path(path).first_or_octet_stream();
        Ok(Response::builder()
            .header("Content-Type", mime_type.as_ref())
            .body(Full::new(Bytes::from(file.data.to_vec())))
            .unwrap())
    } else {
        // Fallback to index.html
        if let Some(file) = Asset::get("index.html") {
            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Full::new(Bytes::from(file.data.to_vec())))
                .unwrap())
        } else {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("404 Not Found")))
                .unwrap())
        }
    }
}
