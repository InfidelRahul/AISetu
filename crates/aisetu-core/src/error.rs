//! Canonical error types used across every AISetu crate.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// High-level classification of a failure, independent of any provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Authentication,
    Authorization,
    RateLimited,
    Timeout,
    Network,
    InvalidRequest,
    ProviderFailure,
    SessionExpired,
    ParseFailure,
    NotFound,
    Conflict,
    Internal,
    Unavailable,
    Cancelled,
    ResourceExhausted,
    Configuration,
    Validation,
}

impl ErrorKind {
    /// OpenAI-compatible error `type` string.
    pub fn openai_type(self) -> &'static str {
        match self {
            Self::Authentication | Self::Authorization | Self::SessionExpired => {
                "invalid_request_error"
            }
            Self::RateLimited | Self::ResourceExhausted => "rate_limit_error",
            Self::Timeout | Self::Network | Self::Unavailable | Self::Cancelled => "api_error",
            Self::InvalidRequest | Self::Validation | Self::NotFound | Self::Conflict => {
                "invalid_request_error"
            }
            Self::ProviderFailure | Self::ParseFailure | Self::Internal | Self::Configuration => {
                "api_error"
            }
        }
    }

    /// HTTP status code that should be returned to API clients.
    pub fn http_status(self) -> u16 {
        match self {
            Self::Authentication | Self::Authorization | Self::SessionExpired => 401,
            Self::RateLimited => 429,
            Self::Timeout => 504,
            Self::Network | Self::Unavailable | Self::Cancelled => 503,
            Self::InvalidRequest | Self::Validation => 400,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::ResourceExhausted => 429,
            Self::ProviderFailure | Self::ParseFailure | Self::Internal | Self::Configuration => {
                500
            }
        }
    }

    /// Whether the caller may reasonably retry the operation.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Timeout
                | Self::Network
                | Self::RateLimited
                | Self::Unavailable
                | Self::ResourceExhausted
        )
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Authentication => "authentication",
            Self::Authorization => "authorization",
            Self::RateLimited => "rate_limited",
            Self::Timeout => "timeout",
            Self::Network => "network",
            Self::InvalidRequest => "invalid_request",
            Self::ProviderFailure => "provider_failure",
            Self::SessionExpired => "session_expired",
            Self::ParseFailure => "parse_failure",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Internal => "internal",
            Self::Unavailable => "unavailable",
            Self::Cancelled => "cancelled",
            Self::ResourceExhausted => "resource_exhausted",
            Self::Configuration => "configuration",
            Self::Validation => "validation",
        };
        f.write_str(s)
    }
}

/// Primary error type for AISetu.
#[derive(Debug, Error)]
#[error("{kind}: {message}")]
pub struct SetuError {
    pub kind: ErrorKind,
    pub message: String,
    pub source_detail: Option<String>,
    pub request_id: Option<String>,
    pub retry_after_ms: Option<u64>,
    pub provider: Option<String>,
}

impl SetuError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source_detail: None,
            request_id: None,
            retry_after_ms: None,
            provider: None,
        }
    }

    pub fn with_source(mut self, detail: impl Into<String>) -> Self {
        self.source_detail = Some(detail.into());
        self
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_retry_after_ms(mut self, ms: u64) -> Self {
        self.retry_after_ms = Some(ms);
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn authentication(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Authentication, message)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Authorization, message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RateLimited, message)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Network, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidRequest, message)
    }

    pub fn provider_failure(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ProviderFailure, message)
    }

    pub fn session_expired(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::SessionExpired, message)
    }

    pub fn parse_failure(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ParseFailure, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Cancelled, message)
    }

    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ResourceExhausted, message)
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Configuration, message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    /// Safe message for API clients; never includes session/credential text.
    pub fn client_message(&self) -> &str {
        &self.message
    }
}

pub type Result<T> = std::result::Result<T, SetuError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_display_and_status() {
        assert_eq!(ErrorKind::Timeout.to_string(), "timeout");
        assert_eq!(ErrorKind::Timeout.http_status(), 504);
        assert!(ErrorKind::Timeout.is_retryable());
        assert!(!ErrorKind::InvalidRequest.is_retryable());
        assert_eq!(ErrorKind::RateLimited.openai_type(), "rate_limit_error");
    }

    #[test]
    fn error_builder() {
        let err = SetuError::timeout("upstream timed out")
            .with_provider("mock")
            .with_request_id("req-1")
            .with_retry_after_ms(250);
        assert_eq!(err.kind, ErrorKind::Timeout);
        assert_eq!(err.provider.as_deref(), Some("mock"));
        assert_eq!(err.request_id.as_deref(), Some("req-1"));
        assert_eq!(err.retry_after_ms, Some(250));
        assert_eq!(err.client_message(), "upstream timed out");
    }
}
