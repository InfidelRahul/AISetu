//! HTTP request types.

use std::time::Duration;

use crate::{headers::HeaderMap, CookieJar, Timeout};

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Request body.
#[derive(Clone, Default, PartialEq, Eq)]
pub enum Body {
    #[default]
    Empty,
    Bytes(Vec<u8>),
    Text(String),
}

impl Body {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(bytes.into())
    }

    pub fn from_text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Empty => &[],
            Self::Bytes(b) => b,
            Self::Text(t) => t.as_bytes(),
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            Self::Bytes(b) => std::str::from_utf8(b).ok(),
            Self::Empty => Some(""),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Body::Empty"),
            Self::Bytes(b) => write!(f, "Body::Bytes(len={})", b.len()),
            Self::Text(t) => write!(f, "Body::Text(len={})", t.len()),
        }
    }
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl From<&str> for Body {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

/// Controlled HTTP request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Body,
    pub timeout: Timeout,
    pub cookies: CookieJar,
}

impl HttpRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(Method::Get, url)
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self::new(Method::Post, url)
    }

    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: HeaderMap::new(),
            body: Body::Empty,
            timeout: Timeout::default(),
            cookies: CookieJar::new(),
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn json(mut self, body: impl Into<String>) -> Self {
        self.headers.set_content_type("application/json");
        self.body = Body::Text(body.into());
        self
    }

    pub fn body(mut self, body: impl Into<Body>) -> Self {
        self.body = body.into();
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Timeout::from_duration(duration);
        self
    }

    pub fn cookie(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.cookies.add(name, value);
        self
    }

    pub fn validate(&self) -> Result<(), aisetu_core::SetuError> {
        if self.url.trim().is_empty() {
            return Err(aisetu_core::SetuError::invalid_request("url is empty"));
        }
        let parsed = url::Url::parse(&self.url).map_err(|e| {
            aisetu_core::SetuError::invalid_request(format!("invalid url '{}': {e}", self.url))
        })?;
        match parsed.scheme() {
            "http" | "https" => Ok(()),
            other => Err(aisetu_core::SetuError::invalid_request(format!(
                "unsupported url scheme '{other}'"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder() {
        let req = HttpRequest::post("https://example.com/v1")
            .header("accept", "application/json")
            .json(r#"{"a":1}"#)
            .cookie("sid", "abc");
        assert_eq!(req.method, Method::Post);
        assert_eq!(req.headers.get("accept"), Some("application/json"));
        assert_eq!(req.body.as_text(), Some(r#"{"a":1}"#));
        assert_eq!(req.cookies.get("sid"), Some("abc"));
        req.validate().unwrap();
    }

    #[test]
    fn rejects_empty_and_bad_scheme() {
        assert!(HttpRequest::get("").validate().is_err());
        assert!(HttpRequest::get("ftp://x").validate().is_err());
        assert!(HttpRequest::get("not a url").validate().is_err());
    }
}
