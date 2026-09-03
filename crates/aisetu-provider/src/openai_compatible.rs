//! Generic OpenAI-compatible upstream adapter.
//!
//! This adapter talks only to documented OpenAI-compatible HTTP APIs. It is useful for
//! self-hosted gateways and providers that intentionally expose that contract.

use std::sync::Arc;

use aisetu_conversation::{ConversationRequest, ConversationResponse, FinishReason, Role, Usage};
use aisetu_session::Session;
use aisetu_transport::{HttpRequest, Transport};
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{
    adapter::{Provider, ProviderId},
    capabilities::ProviderCapabilities,
    error::{normalize_http_error, normalize_transport_error},
};

pub struct OpenAiCompatibleProvider {
    id: ProviderId,
    base_url: String,
    api_key: Option<String>,
    transport: Arc<dyn Transport>,
    capabilities: ProviderCapabilities,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        transport: Arc<dyn Transport>,
    ) -> Self {
        Self {
            id: ProviderId::new(name),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: None,
            transport,
            capabilities: ProviderCapabilities::text_only().with_system_messages().with_streaming(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    fn request_body(&self, request: &ConversationRequest) -> aisetu_core::Result<String> {
        let messages: Vec<Value> = request
            .conversation
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": role_name(m.role),
                    "content": m.content.as_str(),
                })
            })
            .collect();
        let mut body = json!({
            "model": request.model.clone().unwrap_or_default(),
            "messages": messages,
            "stream": false,
        });
        if let Some(t) = request.temperature { body["temperature"] = json!(t); }
        if let Some(n) = request.max_tokens { body["max_tokens"] = json!(n); }
        if !request.stop.is_empty() { body["stop"] = json!(request.stop); }
        serde_json::to_string(&body)
            .map_err(|e| aisetu_core::SetuError::parse_failure(format!("serialize upstream request: {e}")))
    }
}

fn role_name(role: Role) -> &'static str {
    match role { Role::System => "system", Role::User => "user", Role::Assistant => "assistant" }
}

#[async_trait]
impl Provider for OpenAiCompatibleProvider {
    fn id(&self) -> &ProviderId { &self.id }
    fn capabilities(&self) -> &ProviderCapabilities { &self.capabilities }

    async fn complete(
        &self,
        request: ConversationRequest,
        session: Option<&Session>,
    ) -> aisetu_core::Result<ConversationResponse> {
        request.validate()?;
        if session.is_some_and(|s| s.is_expired()) {
            return Err(aisetu_core::SetuError::session_expired("provider session expired"));
        }
        let mut http = HttpRequest::post(format!("{}/chat/completions", self.base_url))
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .json(self.request_body(&request)?);
        if let Some(key) = &self.api_key {
            http = http.header("authorization", format!("Bearer {key}"));
        }
        if let Some(s) = session {
            for (k, v) in s.cookies.iter() { http = http.cookie(k, v); }
            for (k, v) in s.headers.iter() { http = http.header(k, v); }
        }
        let response = self.transport.execute(http).await
            .map_err(|e| normalize_transport_error(self.id.as_str(), e))?;
        if !response.status.is_success() { return Err(normalize_http_error(self.id.as_str(), &response)); }
        let value: Value = response.json()?;
        parse_openai_response(self.id.as_str(), &value, request.model.clone())
    }
}

fn parse_openai_response(provider: &str, value: &Value, model: Option<String>) -> aisetu_core::Result<ConversationResponse> {
    let content = value.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .or_else(|| value.get("text").and_then(Value::as_str))
        .ok_or_else(|| aisetu_core::SetuError::parse_failure("upstream response has no assistant content"))?;
    let finish = match value.pointer("/choices/0/finish_reason").and_then(Value::as_str) {
        Some("length") => FinishReason::Length,
        Some("content_filter") => FinishReason::ContentFilter,
        Some("stop") | None => FinishReason::Stop,
        Some(_) => FinishReason::Stop,
    };
    let mut out = ConversationResponse::assistant(content);
    out.provider = Some(provider.to_string());
    out.model = model.or_else(|| value.get("model").and_then(Value::as_str).map(str::to_string));
    out.finish_reason = finish;
    if let Some(u) = value.get("usage") {
        let prompt = u.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0) as u32;
        let completion = u.get("completion_tokens").and_then(Value::as_u64).unwrap_or(0) as u32;
        out.usage = Some(Usage::new(prompt, completion));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_conversation::{Conversation, Message};
    use aisetu_transport::client::MockTransport;

    #[tokio::test]
    async fn parses_openai_response() {
        let t = Arc::new(MockTransport::json_ok(r#"{"model":"m","choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}]}"#));
        let p = OpenAiCompatibleProvider::new("upstream", "https://example.invalid/v1", t);
        let req = ConversationRequest::new(Conversation::with_messages(vec![Message::user("hi")]));
        let r = p.complete(req, None).await.unwrap();
        assert_eq!(r.text(), "ok");
    }
}
