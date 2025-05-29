use hyper::body::Bytes;
use serde::de::DeserializeOwned;

use crate::structs::GenericError;

/// Ultra-fast JSON body parser with SIMD acceleration
pub async fn extract_json_body<T: DeserializeOwned>(body: Bytes) -> Result<T, GenericError> {
    let mut mutable_bytes = body.to_vec(); // simd-json requires mutable access
    let parsed: T = simd_json::from_slice(&mut mutable_bytes)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    Ok(parsed)
}
