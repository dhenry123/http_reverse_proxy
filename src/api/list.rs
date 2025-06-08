use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header::HeaderValue};
use serde_json::json;
use std::sync::Arc;

use crate::{
    constants::API_HEADER_VALUE_ACCESS_CONTROL_ALLOW_ORIGIN,
    structs::{ApiOjectTypes, GenericError, ProxyConfig},
};

pub async fn api_list_config_object(
    object_type: ApiOjectTypes,
    config: Arc<ProxyConfig>,
) -> Result<Response<Full<Bytes>>, GenericError> {
    let body = match object_type {
        ApiOjectTypes::PoolServers => json!({"pool_servers":config.pool_servers}).to_string(),
        ApiOjectTypes::PoolBackends => json!({"pool_backends":config.pool_backends}).to_string(),
        ApiOjectTypes::Frontends => json!({"frontends":config.frontends}).to_string(),
    };
    // Building response
    let mut response = Response::new(Full::new(Bytes::from(body)));
    response.headers_mut().append(
        hyper::http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );
    response.headers_mut().append(
        hyper::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str(API_HEADER_VALUE_ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
    );
    // Change http code
    *response.status_mut() = StatusCode::OK;
    Ok(response)
}
