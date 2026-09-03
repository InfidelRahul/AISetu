//! Request identifiers used for tracing across layers.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier attached to every inbound API request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(String);

impl RequestId {
    pub fn new() -> Self {
        Self(format!("req_{}", Uuid::new_v4().simple()))
    }

    pub fn from_raw(raw: impl Into<String>) -> Self {
        let s = raw.into();
        if s.is_empty() || s.len() > 128 || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) {
            Self::new()
        } else {
            Self(s)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_raw_falls_back() {
        assert!(RequestId::from_raw("").as_str().starts_with("req_"));
        assert!(RequestId::from_raw("hello world").as_str().starts_with("req_"));
        assert!(RequestId::from_raw("x\nheader").as_str().starts_with("req_"));
        assert!(RequestId::from_raw("x".repeat(129)).as_str().starts_with("req_"));
    }
}
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<RequestId> for String {
    fn from(value: RequestId) -> Self {
        value.0
    }
}

impl AsRef<str> for RequestId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unique_ids() {
        let a = RequestId::new();
        let b = RequestId::new();
        assert_ne!(a, b);
        assert!(a.as_str().starts_with("req_"));
    }

    #[test]
    fn empty_raw_falls_back() {
        let id = RequestId::from_raw("");
        assert!(id.as_str().starts_with("req_"));
    }
}
