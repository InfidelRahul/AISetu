//! Request-id middleware.

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

use aisetu_core::RequestId;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn assign_request_id(mut req: Request, next: Next) -> Response {
    let incoming = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let id = match incoming {
        Some(raw) if !raw.is_empty() => RequestId::from_raw(raw),
        _ => RequestId::new(),
    };
    req.extensions_mut().insert(id.clone());
    let mut resp = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(id.as_str()) {
        resp.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    resp
}
