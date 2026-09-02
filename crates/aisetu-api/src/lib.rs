//! AISetu HTTP API server.
//!
//! Provides the local API foundation and OpenAI-compatible endpoints.

pub mod auth;
pub mod error;
pub mod middleware;
pub mod openai;
pub mod server;
pub mod state;
pub mod streaming;

pub use server::{serve, shutdown_signal, AppHandle};
pub use state::AppState;
