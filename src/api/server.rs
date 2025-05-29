use std::{convert::Infallible, sync::Arc};

use arc_swap::{ArcSwap, ArcSwapAny};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header::HeaderValue};
use serde_json::json;

use crate::structs::{GenericError, ProxyConfig};

use super::{
    body_json_structs::BodyServerActive, json_body::extract_json_body, json_reponse::JsonResponse,
};

pub async fn api_server_active_set(
    config: Arc<ArcSwap<ProxyConfig>>,
    body_bytes: Option<Bytes>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let mut response = Response::new(Full::new(Bytes::from("")));

    let mut body: String = "".to_string();
    let mut status_code = StatusCode::BAD_REQUEST;
    if body_bytes.is_some() {
        //Try to get payload
        let request: Result<BodyServerActive, GenericError> =
            extract_json_body(body_bytes.unwrap()).await;
        match request {
            Ok(request) => {
                // Try to update current config
                let shared_config =
                <Arc<ArcSwapAny<Arc<ProxyConfig>>> as arc_swap::access::Access<ProxyConfig>>::load(&config);
                let mut new_config: ProxyConfig = shared_config.clone();
                let mut server_state: Option<crate::structs::BackendServer> = None; // Will store the modified server if found
                for server in &mut new_config.pool_servers {
                    if server.name == request.name {
                        server.active = request.active;
                        server_state = Some(server.clone());
                        break;
                    }
                }
                // set body response
                body = match server_state {
                    Some(server) => {
                        // Store changes
                        config.store(Arc::new(new_config.clone()));
                        status_code = StatusCode::OK;
                        JsonResponse::success(format!(
                            "Server {} active state set to {}",
                            server.name, server.active
                        ))
                        .with_data(json!({ "server": server }))
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
