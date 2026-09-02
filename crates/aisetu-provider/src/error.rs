//! Normalize provider failures into canonical AISetu errors.

use aisetu_core::{ErrorKind, SetuError};
use aisetu_transport::HttpResponse;
use serde_json::Value;

/// Intermediate provider error before API mapping.
#[derive(Debug, Clone)]
pub struct CanonicalProviderError {
    pub kind: ErrorKind,
    pub message: String,
    pub provider: String,
    pub status: Option<u16>,
}

impl CanonicalProviderError {
    pub fn into_setu(self) -> SetuError {
        let mut err = SetuError::new(self.kind, self.message).with_provider(self.provider);
        if let Some(status) = self.status {
            err = err.with_source(format!("http status {status}"));
        }
        err
    }
}

/// Map an HTTP response that is not a successful conversation payload.
pub fn normalize_http_error(provider: &str, response: &HttpResponse) -> SetuError {
    let status = response.status.as_u16();
    let body = response.text().unwrap_or("");
    let parsed = serde_json::from_str::<Value>(body).ok();
    let message = extract_message(parsed.as_ref(), body, status);

    let kind = match status {
        401 | 403 => {
            if looks_like_session(body) {
                ErrorKind::SessionExpired
            } else {
                ErrorKind::Authentication
            }
        }
        408 | 504 => ErrorKind::Timeout,
        409 => ErrorKind::Conflict,
        413 => ErrorKind::ResourceExhausted,
        422 => ErrorKind::InvalidRequest,
        429 => ErrorKind::RateLimited,
        400 => ErrorKind::InvalidRequest,
        404 => ErrorKind::NotFound,
        500..=599 => ErrorKind::ProviderFailure,
        _ if response.status.is_client_error() => ErrorKind::InvalidRequest,
        _ => ErrorKind::ProviderFailure,
    };

    let mut err = SetuError::new(kind, message)
        .with_provider(provider)
        .with_source(format!("http {status}"));

    if kind == ErrorKind::RateLimited {
        if let Some(retry) = parse_retry_after(response) {
            err = err.with_retry_after_ms(retry);
        }
    }
    err
}

fn extract_message(parsed: Option<&Value>, body: &str, status: u16) -> String {
    if let Some(v) = parsed {
        for path in [
            &["error", "message"][..],
            &["error", "msg"][..],
            &["message"][..],
            &["detail"][..],
            &["error"][..],
        ] {
            if let Some(msg) = walk(v, path) {
                if let Some(s) = msg.as_str() {
                    if !s.is_empty() {
                        return s.to_string();
                    }
                }
            }
        }
    }
    let trimmed = body.trim();
    if !trimmed.is_empty() && trimmed.len() < 300 {
        trimmed.to_string()
    } else {
        format!("provider returned HTTP {status}")
    }
}

fn walk<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key)?;
    }
    Some(cur)
}

fn looks_like_session(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("session") || lower.contains("expired") || lower.contains("login")
}

fn parse_retry_after(response: &HttpResponse) -> Option<u64> {
    let raw = response.header("retry-after")?;
    if let Ok(secs) = raw.parse::<u64>() {
        return Some(secs.saturating_mul(1000));
    }
    None
}

/// Map transport / parse failures that occur around a provider call.
pub fn normalize_transport_error(provider: &str, err: SetuError) -> SetuError {
    err.with_provider(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_transport::{Body, CookieJar, HeaderMap, StatusCode};

    fn resp(status: u16, body: &str) -> HttpResponse {
        HttpResponse {
            status: StatusCode(status),
            headers: HeaderMap::new(),
            body: Body::from_text(body),
            cookies: CookieJar::new(),
            elapsed_ms: 1,
            url: "https://provider.example/v1".into(),
        }
    }

    #[test]
    fn maps_auth_rate_limit_timeout() {
        let e = normalize_http_error("p", &resp(401, r#"{"error":{"message":"bad key"}}"#));
        assert_eq!(e.kind, ErrorKind::Authentication);
        assert_eq!(e.message, "bad key");

        let mut r = resp(429, r#"{"error":{"message":"slow down"}}"#);
        r.headers.insert("retry-after", "2");
        let e = normalize_http_error("p", &r);
        assert_eq!(e.kind, ErrorKind::RateLimited);
        assert_eq!(e.retry_after_ms, Some(2000));

        let e = normalize_http_error("p", &resp(504, "gateway"));
        assert_eq!(e.kind, ErrorKind::Timeout);

        let e = normalize_http_error("p", &resp(401, "session expired, please login"));
        assert_eq!(e.kind, ErrorKind::SessionExpired);
    }
}
