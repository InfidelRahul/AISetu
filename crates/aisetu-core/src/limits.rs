//! Process resource limits used during production hardening.

use serde::{Deserialize, Serialize};

/// Runtime resource limits applied by the API and transport layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum concurrent in-flight API requests.
    pub max_concurrent_requests: usize,
    /// Maximum request body size in bytes.
    pub max_request_bytes: usize,
    /// Maximum response body size in bytes from a provider.
    pub max_response_bytes: usize,
    /// Maximum conversation messages accepted in one request.
    pub max_messages: usize,
    /// Default outbound HTTP timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Graceful shutdown wait in milliseconds.
    pub shutdown_grace_ms: u64,
    /// Maximum open outbound connections.
    pub max_connections: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 128,
            max_request_bytes: 2 * 1024 * 1024,
            max_response_bytes: 8 * 1024 * 1024,
            max_messages: 256,
            request_timeout_ms: 60_000,
            shutdown_grace_ms: 10_000,
            max_connections: 64,
        }
    }
}

impl ResourceLimits {
    /// Validate that all limits are positive and internally consistent.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_concurrent_requests == 0 {
            return Err("max_concurrent_requests must be > 0".into());
        }
        if self.max_request_bytes == 0 {
            return Err("max_request_bytes must be > 0".into());
        }
        if self.max_response_bytes == 0 {
            return Err("max_response_bytes must be > 0".into());
        }
        if self.max_messages == 0 {
            return Err("max_messages must be > 0".into());
        }
        if self.request_timeout_ms == 0 {
            return Err("request_timeout_ms must be > 0".into());
        }
        if self.max_connections == 0 {
            return Err("max_connections must be > 0".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        ResourceLimits::default().validate().unwrap();
    }

    #[test]
    fn zero_rejected() {
        let l = ResourceLimits {
            max_concurrent_requests: 0,
            ..ResourceLimits::default()
        };
        assert!(l.validate().is_err());
    }
}
