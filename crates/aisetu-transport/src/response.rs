//! HTTP response types.

use crate::headers::HeaderMap;
use crate::request::Body;
use crate::CookieJar;

/// HTTP status code wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub fn as_u16(self) -> u16 {
        self.0
    }

    pub fn is_success(self) -> bool {
        (200..300).contains(&self.0)
    }

    pub fn is_client_error(self) -> bool {
        (400..500).contains(&self.0)
    }

    pub fn is_server_error(self) -> bool {
        (500..600).contains(&self.0)
    }

    pub fn is_redirection(self) -> bool {
        (300..400).contains(&self.0)
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Structured HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
    pub cookies: CookieJar,
    /// Elapsed request time in milliseconds.
    pub elapsed_ms: u64,
    pub url: String,
}

impl HttpResponse {
    pub fn text(&self) -> aisetu_core::Result<&str> {
        self.body.as_text().ok_or_else(|| {
            aisetu_core::SetuError::parse_failure("response body is not valid UTF-8")
        })
    }

    pub fn bytes(&self) -> &[u8] {
        self.body.as_bytes()
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> aisetu_core::Result<T> {
        let text = self.text()?;
        serde_json::from_str(text).map_err(|e| {
            aisetu_core::SetuError::parse_failure(format!("failed to parse JSON response: {e}"))
        })
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_classes() {
        assert!(StatusCode(200).is_success());
        assert!(StatusCode(404).is_client_error());
        assert!(StatusCode(503).is_server_error());
        assert!(StatusCode(302).is_redirection());
    }

    #[test]
    fn json_parse() {
        let resp = HttpResponse {
            status: StatusCode(200),
            headers: HeaderMap::new(),
            body: Body::from_text(r#"{"ok":true}"#),
            cookies: CookieJar::new(),
            elapsed_ms: 1,
            url: "https://example.com".into(),
        };
        let v: serde_json::Value = resp.json().unwrap();
        assert_eq!(v["ok"], true);
    }
}
