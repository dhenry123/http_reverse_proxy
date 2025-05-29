use serde_json::{Value, json};

use crate::{
    api::json_datetime::get_iso8601_timestamp,
    constants::{JSON_STATUS_LABEL_ERROR, JSON_STATUS_LABEL_SUCCESS},
};

pub struct JsonResponse {
    inner: Value,
}

impl JsonResponse {
    pub fn success(message: String) -> Self {
        Self {
            inner: json!({
                "status": JSON_STATUS_LABEL_SUCCESS,
                "message": message,
                "timestamp": get_iso8601_timestamp()
            }),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            inner: json!({
                "status": JSON_STATUS_LABEL_ERROR,
                "message": message,
                "timestamp": get_iso8601_timestamp()
            }),
        }
    }

    pub fn error_bad_request() -> Self {
        Self {
            inner: json!({
                "status": JSON_STATUS_LABEL_ERROR,
                "message": format!("Bad request"),
                "timestamp": get_iso8601_timestamp()
            }),
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.inner["data"] = data;
        self
    }

    pub fn with_field<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        self.inner[key] = json!(value);
        self
    }

    pub fn build(self) -> Value {
        self.inner
    }
}

// // Usage:
// let response = JsonResponse::success("User created")
//     .with_data(json!({ "id": 123 }))
//     .with_field("timestamp", chrono::Utc::now())
//     .build();
