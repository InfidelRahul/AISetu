//! API authentication foundation.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use serde::{Deserialize, Serialize};

use aisetu_core::SetuError;

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub authenticated: bool,
}

/// Extractor that enforces the optional API key.
pub struct RequireAuth(pub AuthContext);

impl FromRequestParts<AppState> for RequireAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match &state.config.server.api_key {
            None => Ok(Self(AuthContext {
                authenticated: true,
            })),
            Some(expected) if expected.is_empty() => Ok(Self(AuthContext {
                authenticated: true,
            })),
            Some(expected) => {
                let header = parts
                    .headers
                    .get(AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                let presented = header
                    .strip_prefix("Bearer ")
                    .or_else(|| header.strip_prefix("bearer "))
                    .unwrap_or("");
                if constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
                    Ok(Self(AuthContext {
                        authenticated: true,
                    }))
                } else {
                    Err(ApiError(SetuError::authentication(
                        "invalid or missing API key",
                    )))
                }
            }
        }
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
