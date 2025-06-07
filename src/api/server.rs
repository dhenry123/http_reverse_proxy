use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header::HeaderValue};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{config_manager::ConfigManager, structs::GenericError};

use super::{
    body_json_structs::BodyServerActive, json_body::extract_json_body, json_reponse::JsonResponse,
};

pub async fn api_server_active_set(
    config_manager: Arc<RwLock<ConfigManager>>,
    body_bytes: Option<Bytes>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let mut response = Response::new(Full::new(Bytes::from("")));

    let mut body: String = "".to_string();
    let mut status_code = StatusCode::BAD_REQUEST;
    // Get write lock (async)
    let mut config_manager = config_manager.write().await;

    // payload is provided
    if body_bytes.is_some() {
        //Try to get payload
        let request: Result<BodyServerActive, GenericError> =
            extract_json_body(body_bytes.unwrap()).await;
        match request {
            Ok(request) => {
                // Try to update current config (pool_servers)
                let updated_server = config_manager
                    .set_server_active_state(request.name.clone(), request.active.clone())
                    .await;
                // set body response
                body = match updated_server {
                    Some(_) => {
                        //config.store(Arc::new(new_config.clone()));
                        status_code = StatusCode::OK;
                        JsonResponse::success(format!(
                            "Server {} active state set to {}",
                            request.name.clone(),
                            request.active.clone()
                        ))
                        .with_data(json!({ "server": request.name.clone() }))
                        .build()
                        .to_string()
                    }
                    None => {
                        status_code = StatusCode::NOT_FOUND;
                        JsonResponse::error(format!("Server '{}' not found", request.name))
                            .build()
                            .to_string()
                    }
                };
            }
            Err(_) => body = JsonResponse::error_bad_request().build().to_string(),
        }
    }
    // Set http code
    *response.status_mut() = status_code;
    *response.body_mut() = Full::new(Bytes::from(body));
    response.headers_mut().append(
        hyper::http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );
    Ok(response)
}
