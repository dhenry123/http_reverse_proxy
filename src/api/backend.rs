use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header::HeaderValue};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{
    api::body_json_structs::BodyBackendPost, config_manager::ConfigManager,
    constants::API_HEADER_VALUE_ACCESS_CONTROL_ALLOW_ORIGIN, structs::GenericError,
};

use super::{json_body::extract_json_body, json_reponse::JsonResponse};

pub async fn api_backend_post(
    config_manager: Arc<RwLock<ConfigManager>>,
    body_bytes: Option<Bytes>,
) -> Result<Response<Full<Bytes>>, GenericError> {
    let mut body: String = "".to_string();
    let mut status_code = StatusCode::BAD_REQUEST;
    // Get write lock (async)
    let mut config_manager = config_manager.write().await;

    // payload is provided
    if body_bytes.is_some() {
        //Try to get payload
        let request: Result<BodyBackendPost, GenericError> =
            extract_json_body(body_bytes.unwrap()).await;
        match request {
            Ok(request) => {
                // Try to update current config (pool_servers)
                let changes = config_manager.set_backend(request.clone()).await?;

                // set body response
                body = if changes.iter().count() > 0 {
                    //config.store(Arc::new(new_config.clone()));
                    status_code = StatusCode::OK;
                    JsonResponse::success(format!("Backend has been processed",))
                        .with_data(json!({ "backend": request.clone(),"changes": changes }))
                        .build()
                        .to_string()
                } else {
                    status_code = StatusCode::NOT_FOUND;
                    JsonResponse::error(
                        "The backend has not been processed, check your frontends".to_string(),
                    )
                    .build()
                    .to_string()
                }
            }
            Err(_) => body = JsonResponse::error_bad_request().build().to_string(),
        }
    }

    let mut response = Response::new(Full::new(Bytes::from(body)));
    // Json header and CORS
    response.headers_mut().append(
        hyper::http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );
    response.headers_mut().append(
        hyper::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str(API_HEADER_VALUE_ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
    );
    // Set http code
    *response.status_mut() = status_code;
    Ok(response)
}
