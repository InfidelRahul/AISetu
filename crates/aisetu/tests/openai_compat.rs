//! End-to-end OpenAI-compatible client flow against in-process providers.

use std::sync::Arc;

use aisetu_api::{server::router, AppState};
use aisetu_core::{AppConfig, Shutdown};
use aisetu_provider::{EchoProvider, MockProvider, ModelRegistry, ProviderRegistry, Router};
use aisetu_session::{MemorySessionStore, SessionManager};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn app() -> axum::Router {
    let cfg = AppConfig::load(None).unwrap();
    let mut providers = ProviderRegistry::new();
    providers.register(Arc::new(MockProvider::new("mock")));
    providers.register(Arc::new(EchoProvider::new("echo")));
    let models = ModelRegistry::from_config(&cfg);
    let router_p = Router::new(models, providers);
    let sessions = SessionManager::new(Arc::new(MemorySessionStore::new()));
    let state = AppState::new(cfg, router_p, sessions, Shutdown::new());
    router(state)
}

#[tokio::test]
async fn models_then_chat() {
    let app = app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let models: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ids: Vec<&str> = models["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"aisetu-default"));

    let body = serde_json::json!({
        "model": "aisetu-default",
        "messages": [{"role":"user","content":"hello from client"}]
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let chat: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(chat["object"], "chat.completion");
    assert!(chat["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .contains("hello from client"));
}

#[tokio::test]
async fn invalid_json_is_openai_error() {
    let app = app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_client_error());
}
