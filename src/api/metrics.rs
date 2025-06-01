use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode, header::HeaderValue};
use log::debug;
use serde_json::json;
use tokio_tungstenite::tungstenite::http;

use crate::{
    api::{api_helper::parse_query, json_reponse::JsonResponse},
    state::AppState,
};

pub fn api_metric_get_hits(
    state: Arc<AppState>,
    parts: &http::request::Parts,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let mut response = Response::new(Full::new(Bytes::from("")));

    let mut body: String = "".to_string();
    let mut status_code = StatusCode::OK;

    // Parse query parameters
    match parts.uri.query() {
        Some(q) => {
            let query_params = parse_query(q);
            debug!("query_params: {:?}", query_params);
            //Frontend
            match query_params.get("ft") {
                Some(frontend_name) => {
                    debug!("ft found, value: {}", frontend_name);
                    let hits = state.metrics.get_frontend_count(&frontend_name);
                    match hits {
                        Some(hits) => {
                            body = JsonResponse::success(format!("Metric - Frontend hits",))
                                .with_data(json!({ "frontend":frontend_name, "hits": hits }))
                                .build()
                                .to_string();
                        }
                        None => {
                            body = JsonResponse::error_bad_request().build().to_string();
                        }
                    }
                }
                None => {
                    match query_params.get("dm") {
                        Some(domain_name) => {
                            debug!("ft found, value: {}", domain_name);
                            let hits = state.metrics.get_domain_count(&domain_name);
                            match hits {
                                Some(hits) => {
                                    body = JsonResponse::success(format!("Metric - Domain hits",))
                                        .with_data(json!({ "domain":domain_name, "hits": hits }))
                                        .build()
                                        .to_string();
                                }
                                None => {
                                    body = JsonResponse::error_bad_request().build().to_string();
                                }
                            }
                        }
                        None => {
                            match query_params.get("sv") {
                                Some(server_name) => {
                                    debug!("ft found, value: {}", server_name);
                                    let hits = state.metrics.get_server_count(&server_name);
                                    match hits {
                                        Some(hits) => {
                                            body = JsonResponse::success(format!(
                                                "Metric - Server hits",
                                            ))
                                            .with_data(
                                                json!({ "server":server_name, "hits": hits }),
                                            )
                                            .build()
                                            .to_string();
                                        }
                                        None => {
                                            body = JsonResponse::error_bad_request()
                                                .build()
                                                .to_string();
                                        }
                                    }
                                }
                                None => {
                                    status_code = StatusCode::BAD_REQUEST;
                                    body = JsonResponse::error_bad_request().build().to_string();
                                }
                            };
                        }
                    };
                }
            };
        }
        None => {
            status_code = StatusCode::BAD_REQUEST;
            body = JsonResponse::error_bad_request().build().to_string();
        }
    };

    *response.status_mut() = status_code;
    *response.body_mut() = Full::new(Bytes::from(body));
    response.headers_mut().append(
        hyper::http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );
    Ok(response)
}
