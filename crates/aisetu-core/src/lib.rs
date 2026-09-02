//! AISetu core foundation.
//!
//! Provides shared error types, configuration loading, request identifiers,
//! and tracing initialization. This crate has no HTTP, provider, or API
//! dependencies.

pub mod config;
pub mod error;
pub mod id;
pub mod limits;
pub mod logging;
pub mod redact;
pub mod shutdown;

pub use config::{AppConfig, ConfigError, ConfigSource};
pub use error::{ErrorKind, Result, SetuError};
pub use id::RequestId;
pub use limits::ResourceLimits;
pub use logging::init_tracing;
pub use shutdown::Shutdown;

/// Crate version of the AISetu core library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name.
pub const PRODUCT: &str = "AISetu";
