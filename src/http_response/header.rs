use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, header::HeaderValue};

pub fn header_add_no_cache(response: &mut Response<Full<Bytes>>) {
    response.headers_mut().append(
        hyper::http::header::CACHE_CONTROL,
        HeaderValue::from_str("no-store,no-cache, must-revalidate").unwrap(),
    );
    response.headers_mut().append(
        hyper::http::header::PRAGMA,
        HeaderValue::from_str("no-cache").unwrap(),
    );
    response.headers_mut().append(
        hyper::http::header::EXPIRES,
        HeaderValue::from_str("0").unwrap(),
    );
}
