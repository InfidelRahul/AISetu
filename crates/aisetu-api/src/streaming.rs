//! SSE streaming for chat completions.
//!
//! Provider returns a complete response; the API emits OpenAI-style SSE chunks.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse,
};
use chrono::Utc;
use serde::Serialize;

use aisetu_conversation::ConversationRequest;
use aisetu_provider::Provider;
use aisetu_session::Session;

use crate::{error::ApiError, openai::finish_reason_str, state::AppState};

#[derive(Debug, Serialize)]
struct StreamChoice {
    index: u32,
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct StreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Serialize)]
struct StreamChunk {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<StreamChoice>,
}

pub async fn chat_completion_stream(
    _state: AppState,
    provider: Arc<dyn Provider>,
    request: ConversationRequest,
    session: Option<Session>,
    model: String,
) -> Result<axum::response::Response, ApiError> {
    let id = format!(
        "chatcmpl_{}",
        aisetu_core::RequestId::new()
            .as_str()
            .trim_start_matches("req_")
    );
    let created = Utc::now().timestamp();

    let mut stream = provider
        .stream(request, session.as_ref())
        .await
        .map_err(ApiError::from)?;

    let id_for_stream = id.clone();
    let model_for_stream = model.clone();
    let output = async_stream(stream, id_for_stream, created, model_for_stream);

    let sse = Sse::new(output).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );
    Ok(sse.into_response())
}

fn async_stream(
    provider_stream: futures::stream::BoxStream<'static, aisetu_core::Result<String>>,
    id: String,
    created: i64,
    model: String,
) -> impl futures::Stream<Item = Result<Event, Infallible>> {
    let role = StreamChunk {
        id: id.clone(), object: "chat.completion.chunk".into(), created, model: model.clone(),
        choices: vec![StreamChoice { index: 0, delta: StreamDelta { role: Some("assistant".into()), content: None }, finish_reason: None }],
    };
    let prefix = futures::stream::iter(vec![Ok(Event::default().data(serde_json::to_string(&role).unwrap_or_else(|_| "{}".into())))]);

    let body = futures::stream::unfold(
        (Some(provider_stream), false),
        move |(state, finished)| {
            let id = id.clone();
            let model = model.clone();
            async move {
                if finished { return None; }
                let mut stream = state?;
                match futures::StreamExt::next(&mut stream).await {
                    Some(Ok(text)) => {
                        let chunk = StreamChunk {
                            id, object: "chat.completion.chunk".into(), created, model,
                            choices: vec![StreamChoice { index: 0, delta: StreamDelta { role: None, content: Some(text) }, finish_reason: None }],
                        };
                        let event = Event::default().data(serde_json::to_string(&chunk).unwrap_or_else(|_| "{}".into()));
                        Some((Ok(event), (Some(stream), false)))
                    }
                    Some(Err(err)) => {
                        let body = crate::error::OpenAiErrorBody::from(&err);
                        let event = Event::default().event("error").data(serde_json::to_string(&body).unwrap_or_else(|_| "{}".into()));
                        Some((Ok(event), (None, true)))
                    }
                    None => {
                        let done = StreamChunk {
                            id, object: "chat.completion.chunk".into(), created, model,
                            choices: vec![StreamChoice { index: 0, delta: StreamDelta { role: None, content: None }, finish_reason: Some("stop".into()) }],
                        };
                        let event = Event::default().data(serde_json::to_string(&done).unwrap_or_else(|_| "{}".into()));
                        Some((Ok(event), (None, true)))
                    }
                }
            }
        },
    );

    prefix.chain(body).chain(futures::stream::once(async { Ok(Event::default().data("[DONE]")) }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn stream_module_compiles() {
        assert!(true);
    }
}
