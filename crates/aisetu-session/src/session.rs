//! Session types. Values are never printed in Debug.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new() -> Self {
        Self(format!("sess_{}", Uuid::new_v4().simple()))
    }

    pub fn from_raw(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionId([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Invalid,
    Expired,
}

/// Provider session: cookies and headers required by the transport layer.
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub provider: String,
    pub cookies: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(provider: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            provider: provider.into(),
            cookies: BTreeMap::new(),
            headers: BTreeMap::new(),
            state: SessionState::Active,
            created_at: now,
            updated_at: now,
            expires_at: Some(now + Duration::hours(24)),
        }
    }

    pub fn cookie(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.cookies.insert(name.into(), value.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.expires_at = Some(Utc::now() + ttl);
        self
    }

    pub fn is_expired(&self) -> bool {
        if self.state != SessionState::Active {
            return true;
        }
        match self.expires_at {
            Some(exp) => Utc::now() >= exp,
            None => false,
        }
    }

    pub fn validate(&self) -> aisetu_core::Result<()> {
        if self.provider.trim().is_empty() {
            return Err(aisetu_core::SetuError::validation(
                "session provider is empty",
            ));
        }
        if self.state == SessionState::Invalid {
            return Err(aisetu_core::SetuError::session_expired(
                "session has been invalidated",
            ));
        }
        if self.is_expired() {
            return Err(aisetu_core::SetuError::session_expired(format!(
                "session for provider '{}' has expired",
                self.provider
            )));
        }
        Ok(())
    }

    pub fn invalidate(&mut self) {
        self.state = SessionState::Invalid;
        for value in self.cookies.values_mut() {
            value.clear();
        }
        for value in self.headers.values_mut() {
            value.clear();
        }
        self.cookies.clear();
        self.headers.clear();
        self.updated_at = Utc::now();
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("provider", &self.provider)
            .field("state", &self.state)
            .field("cookie_names", &self.cookies.keys().collect::<Vec<_>>())
            .field("header_names", &self.headers.keys().collect::<Vec<_>>())
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_active() {
        let s = Session::new("mock").cookie("sid", "abc");
        s.validate().unwrap();
        assert!(!s.is_expired());
    }

    #[test]
    fn expired_ttl() {
        let s = Session::new("mock").with_ttl(Duration::milliseconds(-1));
        assert!(s.is_expired());
        assert!(s.validate().is_err());
    }

    #[test]
    fn invalidate_clears_secrets() {
        let mut s = Session::new("mock").cookie("sid", "abc");
        s.invalidate();
        assert!(s.cookies.is_empty());
        assert_eq!(s.state, SessionState::Invalid);
        assert!(s.validate().is_err());
    }

    #[test]
    fn debug_redacts() {
        let s = Session::new("mock").cookie("sid", "super-secret");
        let d = format!("{s:?}");
        assert!(!d.contains("super-secret"));
        assert!(d.contains("sid"));
    }
}
