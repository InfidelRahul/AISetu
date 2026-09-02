//! API error framework mapped onto OpenAI-style JSON.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use aisetu_core::{ErrorKind, SetuError};

#[derive(Debug, Serialize)]
pub struct OpenAiErrorBody {
    pub error: OpenAiErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct OpenAiErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: String,
}

impl From<&SetuError> for OpenAiErrorBody {
    fn from(err: &SetuError) -> Self {
        Self {
            error: OpenAiErrorDetail {
                message: err.client_message().to_string(),
                error_type: err.kind.openai_type().to_string(),
                param: None,
                code: err.kind.to_string(),
            },
        }
    }
}

#[derive(Debug)]
pub struct ApiError(pub SetuError);

impl From<SetuError> for ApiError {
    fn from(value: SetuError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.kind.http_status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (status, Json(OpenAiErrorBody::from(&self.0))).into_response();
        if let Some(id) = &self.0.request_id {
            if let Ok(value) = axum::http::HeaderValue::from_str(id) {
                response.headers_mut().insert("x-request-id", value);
            }
        }
        if let Some(ms) = self.0.retry_after_ms {
            let secs = (ms / 1000).max(1);
            if let Ok(value) = axum::http::HeaderValue::from_str(&secs.to_string()) {
                response.headers_mut().insert("retry-after", value);
            }
        }
        tracing::warn!(
            kind = %self.0.kind,
            status = status.as_u16(),
            provider = self.0.provider.as_deref().unwrap_or("-"),
            "api error"
        );
        response
    }
}

pub fn map_kind_status(kind: ErrorKind) -> StatusCode {
    StatusCode::from_u16(kind.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_shape() {
        let err = SetuError::rate_limited("slow").with_retry_after_ms(2000);
        let body = OpenAiErrorBody::from(&err);
        assert_eq!(body.error.code, "rate_limited");
        assert_eq!(body.error.error_type, "rate_limit_error");
    }
}
