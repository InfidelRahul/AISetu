//! OpenAI-compatible request and response types, plus handlers.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use aisetu_conversation::{Conversation, ConversationRequest, FinishReason, Message, Role};
use aisetu_core::SetuError;
use aisetu_provider::Capability;

use crate::{
    auth::RequireAuth, error::ApiError, state::AppState, streaming::chat_completion_stream,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    pub object: String,
    pub data: Vec<ModelObject>,
}

pub async fn list_models(
    State(state): State<AppState>,
    RequireAuth(_): RequireAuth,
) -> Result<Json<ModelList>, ApiError> {
    let created = Utc::now().timestamp();
    let data = state
        .router
        .models
        .list()
        .iter()
        .map(|m| ModelObject {
            id: m.id.clone(),
            object: "model".into(),
            created,
            owned_by: m.owned_by.clone(),
        })
        .collect();
    Ok(Json(ModelList {
        object: "list".into(),
        data,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: ChatCompletionUsage,
}

pub async fn chat_completions(
    State(state): State<AppState>,
    RequireAuth(_): RequireAuth,
    Json(body): Json<ChatCompletionRequest>,
) -> Result<axum::response::Response, ApiError> {
    if state.shutdown.is_triggered() {
        return Err(ApiError(SetuError::unavailable("server is shutting down")));
    }
    if body.messages.len() > state.config.limits.max_messages {
        return Err(ApiError(SetuError::invalid_request(format!(
            "too many messages (max {})",
            state.config.limits.max_messages
        ))));
    }

    let conversation = openai_to_conversation(&body)?;
    conversation.validate().map_err(ApiError::from)?;

    let mut request = ConversationRequest::new(conversation);
    request.model = Some(body.model.clone());
    request.temperature = body.temperature;
    request.max_tokens = body.max_tokens;
    request.stop = body.stop.clone().unwrap_or_default();
    request.stream = body.stream;
    request.validate().map_err(ApiError::from)?;

    let cap = if body.stream {
        Capability::Streaming
    } else {
        Capability::Text
    };
    let (record, provider) = state
        .router
        .require(&body.model, cap)
        .map_err(ApiError::from)?;

    if !provider.capabilities().system_messages
        && request.conversation.system_messages().next().is_some()
    {
        return Err(ApiError(SetuError::invalid_request(format!(
            "model '{}' does not support system messages",
            body.model
        ))));
    }

    let session = state
        .sessions
        .for_transport(&record.provider)
        .await
        .map_err(ApiError::from)?;

    let upstream_model = record
        .upstream_model
        .clone()
        .unwrap_or_else(|| body.model.clone());
    request.model = Some(upstream_model);

    if body.stream {
        return chat_completion_stream(state, provider, request, session, body.model).await;
    }

    let started = std::time::Instant::now();
    let response = provider
        .complete(request, session.as_ref())
        .await
        .map_err(ApiError::from)?;
    tracing::info!(
        model = %body.model,
        provider = %record.provider,
        conversation_ms = started.elapsed().as_millis() as u64,
        "chat completion"
    );

    let usage = response.usage.clone().unwrap_or_else(|| {
        aisetu_conversation::Usage::new(0, response.text().split_whitespace().count() as u32)
    });
    let payload = ChatCompletionResponse {
        id: format!(
            "chatcmpl_{}",
            aisetu_core::RequestId::new()
                .as_str()
                .trim_start_matches("req_")
        ),
        object: "chat.completion".into(),
        created: Utc::now().timestamp(),
        model: body.model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".into(),
                content: response.text().to_string(),
            },
            finish_reason: finish_reason_str(response.finish_reason).into(),
        }],
        usage: ChatCompletionUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        },
    };
    Ok(Json(payload).into_response())
}

pub fn openai_to_conversation(body: &ChatCompletionRequest) -> Result<Conversation, ApiError> {
    if body.messages.is_empty() {
        return Err(ApiError(SetuError::invalid_request(
            "messages must not be empty",
        )));
    }
    let mut messages = Vec::with_capacity(body.messages.len());
    for msg in &body.messages {
        let role = Role::parse(&msg.role).map_err(ApiError::from)?;
        messages.push(Message::new(role, msg.content.clone()));
    }
    Ok(Conversation::with_messages(messages))
}

pub fn finish_reason_str(reason: FinishReason) -> &'static str {
    match reason {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::Error => "stop",
    }
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": if state.shutdown.is_triggered() { "stopping" } else { "ok" },
        "product": aisetu_core::PRODUCT,
        "version": aisetu_core::VERSION,
        "uptime_ms": state.started_at.elapsed().as_millis() as u64,
    });
    (StatusCode::OK, Json(body))
}

pub async fn not_found() -> impl IntoResponse {
    ApiError(SetuError::not_found("route not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_messages() {
        let body = ChatCompletionRequest {
            model: "aisetu-default".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "s".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "hi".into(),
                },
            ],
            temperature: None,
            max_tokens: None,
            stream: false,
            stop: None,
        };
        let c = openai_to_conversation(&body).unwrap();
        assert_eq!(c.len(), 2);
        c.validate().unwrap();
    }

    #[test]
    fn rejects_unknown_role() {
        let body = ChatCompletionRequest {
            model: "x".into(),
            messages: vec![ChatMessage {
                role: "tool".into(),
                content: "x".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            stop: None,
        };
        assert!(openai_to_conversation(&body).is_err());
    }
}
