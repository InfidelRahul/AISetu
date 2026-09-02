//! Shared API application state.

use std::sync::Arc;
use std::time::Instant;

use aisetu_core::{AppConfig, Shutdown};
use aisetu_provider::Router;
use aisetu_session::SessionManager;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub router: Arc<Router>,
    pub sessions: Arc<SessionManager>,
    pub shutdown: Shutdown,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        router: Router,
        sessions: SessionManager,
        shutdown: Shutdown,
    ) -> Self {
        Self {
            config: Arc::new(config),
            router: Arc::new(router),
            sessions: Arc::new(sessions),
            shutdown,
            started_at: Instant::now(),
        }
    }
}
