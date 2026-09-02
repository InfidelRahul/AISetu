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
use futures::stream;
use serde::Serialize;
use tokio_stream::StreamExt;

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
    let response = provider
        .complete(request, session.as_ref())
        .await
        .map_err(ApiError::from)?;

    let id = format!(
        "chatcmpl_{}",
        aisetu_core::RequestId::new()
            .as_str()
            .trim_start_matches("req_")
    );
    let created = Utc::now().timestamp();
    let text = response.text().to_string();
    let finish = finish_reason_str(response.finish_reason).to_string();

    let chunks = chunk_text(&text, 24);
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();

    let role_chunk = StreamChunk {
        id: id.clone(),
        object: "chat.completion.chunk".into(),
        created,
        model: model.clone(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: Some("assistant".into()),
                content: Some(String::new()),
            },
            finish_reason: None,
        }],
    };
    events.push(Ok(
        Event::default().data(serde_json::to_string(&role_chunk).unwrap())
    ));

    for part in chunks {
        let chunk = StreamChunk {
            id: id.clone(),
            object: "chat.completion.chunk".into(),
            created,
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(part),
                },
                finish_reason: None,
            }],
        };
        events.push(Ok(
            Event::default().data(serde_json::to_string(&chunk).unwrap())
        ));
    }

    let done_chunk = StreamChunk {
        id,
        object: "chat.completion.chunk".into(),
        created,
        model,
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: None,
            },
            finish_reason: Some(finish),
        }],
    };
    events.push(Ok(
        Event::default().data(serde_json::to_string(&done_chunk).unwrap())
    ));
    events.push(Ok(Event::default().data("[DONE]")));

    let sse = Sse::new(stream::iter(events).throttle(Duration::from_millis(0))).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );
    Ok(sse.into_response())
}

fn chunk_text(text: &str, size: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if buf.chars().count() >= size {
            out.push(std::mem::take(&mut buf));
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks() {
        let parts = chunk_text("abcdefghij", 3);
        assert_eq!(parts, vec!["abc", "def", "ghi", "j"]);
    }
}
