//! HTTP server, routing, and graceful shutdown.

use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use aisetu_core::Shutdown;

use crate::{
    middleware::assign_request_id,
    openai::{chat_completions, health, list_models, not_found},
    state::AppState,
};

pub struct AppHandle {
    pub addr: SocketAddr,
}

pub fn router(state: AppState) -> Router {
    let timeout = Duration::from_millis(state.config.limits.request_timeout_ms);
    let body_limit = state.config.limits.max_request_bytes;

    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .fallback(not_found)
        .layer(axum::middleware::from_fn(assign_request_id))
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(ConcurrencyLimitLayer::new(
            state.config.limits.max_concurrent_requests,
        ))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            timeout,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
                let id = req
                    .extensions()
                    .get::<aisetu_core::RequestId>()
                    .map(|i| i.as_str().to_string())
                    .unwrap_or_default();
                tracing::info_span!(
                    "http.api",
                    method = %req.method(),
                    path = %req.uri().path(),
                    request_id = %id,
                )
            }),
        )
        .layer(CorsLayer::new())
        .with_state(state)
}

pub async fn serve(state: AppState) -> aisetu_core::Result<AppHandle> {
    let addr: SocketAddr = state.config.server.listen_addr().parse().map_err(|e| {
        aisetu_core::SetuError::configuration(format!("invalid listen address: {e}"))
    })?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| aisetu_core::SetuError::unavailable(format!("failed to bind {addr}: {e}")))?;
    let local = listener
        .local_addr()
        .map_err(|e| aisetu_core::SetuError::internal(format!("local_addr: {e}")))?;
    tracing::info!(%local, "AISetu API listening");

    let shutdown = state.shutdown.clone();
    let grace = Duration::from_millis(state.config.limits.shutdown_grace_ms);
    let app = router(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            wait_for_shutdown(&shutdown).await;
            tracing::info!(grace_ms = grace.as_millis() as u64, "graceful shutdown");
            tokio::time::sleep(Duration::from_millis(10)).await;
        })
        .await
        .map_err(|e| aisetu_core::SetuError::internal(format!("server error: {e}")))?;

    Ok(AppHandle { addr: local })
}

pub async fn shutdown_signal(shutdown: Shutdown) {
    wait_for_shutdown(&shutdown).await;
}

async fn wait_for_shutdown(shutdown: &Shutdown) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => shutdown.trigger(),
        _ = terminate => shutdown.trigger(),
        _ = wait_flag(shutdown) => {}
    }
}

async fn wait_flag(shutdown: &Shutdown) {
    while !shutdown.is_triggered() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use aisetu_core::AppConfig;
    use aisetu_provider::{
        EchoProvider, MockProvider, ModelRegistry, ProviderRegistry, Router as PRouter,
    };
    use aisetu_session::{MemorySessionStore, SessionManager};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let cfg = AppConfig::load(None).unwrap();
        let mut providers = ProviderRegistry::new();
        providers.register(Arc::new(MockProvider::new("mock")));
        providers.register(Arc::new(EchoProvider::new("echo")));
        let models = ModelRegistry::from_config(&cfg);
        let router = PRouter::new(models, providers);
        let sessions = SessionManager::new(Arc::new(MemorySessionStore::new()));
        AppState::new(cfg, router, sessions, Shutdown::new())
    }

    #[tokio::test]
    async fn health_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get("x-request-id").is_some());
    }

    #[tokio::test]
    async fn models_list() {
        let app = router(test_state());
        let resp = app
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
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["object"], "list");
        assert!(v["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "aisetu-default"));
    }

    #[tokio::test]
    async fn chat_completion() {
        let app = router(test_state());
        let body = serde_json::json!({
            "model": "aisetu-default",
            "messages": [{"role":"user","content":"hello"}]
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
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["object"], "chat.completion");
        assert_eq!(v["choices"][0]["message"]["role"], "assistant");
        let content = v["choices"][0]["message"]["content"].as_str().unwrap();
        assert!(content.contains("hello"));
    }

    #[tokio::test]
    async fn chat_echo_provider_via_model() {
        let app = router(test_state());
        let body = serde_json::json!({
            "model": "aisetu-echo",
            "messages": [{"role":"user","content":"ping"}]
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
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["choices"][0]["message"]["content"], "ping");
    }

    #[tokio::test]
    async fn unknown_model() {
        let app = router(test_state());
        let body = serde_json::json!({
            "model": "does-not-exist",
            "messages": [{"role":"user","content":"hi"}]
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
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn empty_messages_rejected() {
        let app = router(test_state());
        let body = serde_json::json!({
            "model": "aisetu-default",
            "messages": []
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
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_route() {
        let app = router(test_state());
        let resp = app
            .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn streaming_sse() {
        let app = router(test_state());
        let body = serde_json::json!({
            "model": "aisetu-default",
            "stream": true,
            "messages": [{"role":"user","content":"hello world"}]
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
        let ctype = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(ctype.contains("text/event-stream"));
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("chat.completion.chunk"));
        assert!(text.contains("[DONE]"));
        assert!(text.contains("hello"));
    }

    #[tokio::test]
    async fn auth_required_when_configured() {
        let mut cfg = AppConfig::load(None).unwrap();
        cfg.server.api_key = Some("secret-key".into());
        let mut providers = ProviderRegistry::new();
        providers.register(Arc::new(MockProvider::new("mock")));
        providers.register(Arc::new(EchoProvider::new("echo")));
        let models = ModelRegistry::from_config(&cfg);
        let router_p = PRouter::new(models, providers);
        let sessions = SessionManager::new(Arc::new(MemorySessionStore::new()));
        let state = AppState::new(cfg, router_p, sessions, Shutdown::new());
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .header("authorization", "Bearer secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
