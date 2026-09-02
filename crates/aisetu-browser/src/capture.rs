//! Capture of permitted session state from a browser login.

use std::collections::BTreeMap;

use aisetu_session::Session;
use serde::{Deserialize, Serialize};

/// Permitted cookies/headers captured after login.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapturedSession {
    pub provider: String,
    pub cookies: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
}

impl CapturedSession {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            cookies: BTreeMap::new(),
            headers: BTreeMap::new(),
        }
    }

    pub fn into_session(self) -> Session {
        let mut session = Session::new(self.provider);
        session.cookies = self.cookies;
        session.headers = self.headers;
        session
    }
}

/// Policy describing which cookie/header names may be transferred.
#[derive(Debug, Clone)]
pub struct SessionCapture {
    pub allowed_cookie_names: Vec<String>,
    pub allowed_header_names: Vec<String>,
}

impl SessionCapture {
    pub fn permissive() -> Self {
        Self {
            allowed_cookie_names: Vec::new(),
            allowed_header_names: Vec::new(),
        }
    }

    pub fn allow_cookies(names: &[&str]) -> Self {
        Self {
            allowed_cookie_names: names.iter().map(|s| (*s).to_string()).collect(),
            allowed_header_names: vec!["authorization".into()],
        }
    }

    pub fn filter(&self, mut captured: CapturedSession) -> CapturedSession {
        if !self.allowed_cookie_names.is_empty() {
            captured.cookies.retain(|k, _| {
                self.allowed_cookie_names
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(k))
            });
        }
        if !self.allowed_header_names.is_empty() {
            captured.headers.retain(|k, _| {
                self.allowed_header_names
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(k))
            });
        }
        captured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_cookies() {
        let mut cap = CapturedSession::new("web");
        cap.cookies.insert("sid".into(), "abc".into());
        cap.cookies.insert("tracking".into(), "nope".into());
        let filtered = SessionCapture::allow_cookies(&["sid"]).filter(cap);
        assert_eq!(filtered.cookies.len(), 1);
        assert_eq!(filtered.cookies.get("sid").map(String::as_str), Some("abc"));
        let session = filtered.into_session();
        assert_eq!(session.provider, "web");
    }
}
