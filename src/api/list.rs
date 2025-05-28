use arc_swap::{ArcSwap, ArcSwapAny};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header::HeaderValue};
use serde_json::json;
use std::{convert::Infallible, sync::Arc};

use crate::structs::{ApiOjectTypes, ProxyConfig};

pub async fn list_config_object(
    object_type: ApiOjectTypes,
    config: Arc<ArcSwap<ProxyConfig>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let config =
        <Arc<ArcSwapAny<Arc<ProxyConfig>>> as arc_swap::access::Access<ProxyConfig>>::load(&config);
    let body = match object_type {
        ApiOjectTypes::PoolServers => json!({"servers":config.pool_servers}).to_string(),
        ApiOjectTypes::PoolBackend => json!({"servers":config.pool_backends}).to_string(),
    };
    //println!("body: {:?}", body);
    let mut response = Response::new(Full::new(Bytes::from(body)));
    response.headers_mut().append(
        hyper::http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );
    // Change http code
    *response.status_mut() = StatusCode::OK;
    Ok(response)
}
