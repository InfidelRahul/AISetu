//! Tracing / logging foundation.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::LogFormat;

/// Initialize the global tracing subscriber.
///
/// Safe to call once at process start. Subsequent calls are ignored.
pub fn init_tracing(level: &str, format: LogFormat) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);

    match format {
        LogFormat::Json => {
            let _ = registry
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_current_span(true)
                        .with_span_list(false)
                        .flatten_event(true),
                )
                .try_init();
        }
        LogFormat::Pretty => {
            let _ = registry
                .with(
                    fmt::layer()
                        .compact()
                        .with_target(true)
                        .with_ansi(true)
                        .with_thread_ids(false),
                )
                .try_init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        init_tracing("debug", LogFormat::Pretty);
        init_tracing("info", LogFormat::Json);
    }
}
