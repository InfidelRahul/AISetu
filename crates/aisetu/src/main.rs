//! AISetu process entrypoint.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};

use aisetu_api::{serve, AppState};
use aisetu_browser::{transfer_to_manager, BrowserPlugin, HeadlessScriptBrowser, SessionCapture};
use aisetu_core::{init_tracing, AppConfig, Shutdown};
use aisetu_provider::{
    EchoProvider, HttpJsonProvider, MockProvider, OpenAiCompatibleProvider, ModelRegistry, ProviderRegistry, Router,
};
use aisetu_session::{FileSessionStore, SecretStore, SessionManager};
use aisetu_transport::HttpTransport;

#[derive(Parser, Debug)]
#[command(name = "aisetu", version, about = "AISetu conversation bridge")]
struct Cli {
    /// Path to aisetu.toml
    #[arg(global = true, short, long, env = "AISETU_CONFIG")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the local OpenAI-compatible API server
    Serve,
    /// Print the resolved configuration (secrets redacted)
    Config,
    /// Bootstrap a provider session via the browser plugin
    Login {
        /// Provider name
        #[arg(long)]
        provider: String,
        /// Login URL
        #[arg(long)]
        url: String,
        /// Cookie name=value to inject (headless / scripted login)
        #[arg(long)]
        cookie: Option<String>,
    },
    /// Show version and build information
    Version,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("aisetu: {err}");
        std::process::exit(1);
    }
}

async fn run() -> aisetu_core::Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load(cli.config.as_deref())?;
    init_tracing(&config.log.level, config.log.format);

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => cmd_serve(config).await,
        Command::Config => {
            println!("bind = {}", config.server.listen_addr());
            println!("log.level = {}", config.log.level);
            println!(
                "providers = {}",
                config
                    .providers
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!(
                "models = {}",
                config
                    .models
                    .iter()
                    .map(|m| m.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!(
                "api_key = {}",
                if config.server.api_key.is_some() {
                    "[SET]"
                } else {
                    "[UNSET]"
                }
            );
            Ok(())
        }
        Command::Login {
            provider,
            url,
            cookie,
        } => cmd_login(config, provider, url, cookie).await,
        Command::Version => {
            println!("{} {}", aisetu_core::PRODUCT, aisetu_core::VERSION);
            println!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
            Ok(())
        }
    }
}

async fn cmd_serve(config: AppConfig) -> aisetu_core::Result<()> {
    let state = build_state(config)?;
    tracing::info!(
        bind = %state.config.server.listen_addr(),
        "starting AISetu"
    );
    let _ = serve(state).await?;
    Ok(())
}

async fn cmd_login(
    config: AppConfig,
    provider: String,
    url: String,
    cookie: Option<String>,
) -> aisetu_core::Result<()> {
    let secrets_dir = config.storage.resolve_secrets_dir();
    let _secrets = SecretStore::open(secrets_dir.join("vault"))?;
    let store = FileSessionStore::new(secrets_dir.join("provider-sessions"))?;
    let manager = SessionManager::new(Arc::new(store));

    let browser = HeadlessScriptBrowser::new(move |_url, provider| {
        let mut cap = aisetu_browser::CapturedSession::new(provider);
        if let Some(ref c) = cookie {
            if let Some((k, v)) = c.split_once('=') {
                cap.cookies.insert(k.to_string(), v.to_string());
            }
        }
        Ok(cap)
    });
    let captured = browser
        .authenticate(&url, &provider, &SessionCapture::allow_cookies(&["sid", "session", "sessionid", "auth"]))
        .await?;
    if captured.cookies.is_empty() && captured.headers.is_empty() {
        return Err(aisetu_core::SetuError::authentication(
            "browser plugin did not capture any session state; pass --cookie name=value",
        ));
    }
    let session = transfer_to_manager(&manager, captured).await?;
    tracing::info!(provider = %session.provider, "session stored");
    println!("session stored for provider '{}'", session.provider);
    Ok(())
}

fn build_state(config: AppConfig) -> aisetu_core::Result<AppState> {
    config
        .limits
        .validate()
        .map_err(aisetu_core::SetuError::configuration)?;

    let secrets_dir = config.storage.resolve_secrets_dir();
    let data_dir = config.storage.resolve_data_dir();
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| aisetu_core::SetuError::configuration(format!("data dir: {e}")))?;
    let secret_store = SecretStore::open(secrets_dir.join("vault"))?;
    if let Some(key) = config.server.api_key.as_deref() {
        let secrets = SecretStore::split_from_config(Some(key), Default::default());
        for s in secrets {
            secret_store.put(&s)?;
        }
    }

    let session_store = FileSessionStore::new(secrets_dir.join("provider-sessions"))?;
    let sessions = SessionManager::new(Arc::new(session_store));

    let transport = Arc::new(HttpTransport::with_limits(
        &config.transport,
        config.limits.max_response_bytes,
    )?);

    let mut providers = ProviderRegistry::new();
    for entry in &config.providers {
        match entry.kind.as_str() {
            "mock" => {
                providers.register(Arc::new(MockProvider::new(entry.name.clone())));
            }
            "echo" => {
                providers.register(Arc::new(EchoProvider::new(entry.name.clone())));
            }
            "http" | "http_json" => {
                if !entry.enabled {
                    continue;
                }
                let base = entry.base_url.clone().ok_or_else(|| {
                    aisetu_core::SetuError::configuration(format!(
                        "provider '{}' requires base_url",
                        entry.name
                    ))
                })?;
                let mut p = HttpJsonProvider::new(entry.name.clone(), base, transport.clone());
                if let Some(model) = &entry.model {
                    p = p.with_model(model.clone());
                }
                providers.register(Arc::new(p));
            }
            "openai_compatible" => {
                if !entry.enabled { continue; }
                let base = entry.base_url.clone().ok_or_else(|| {
                    aisetu_core::SetuError::configuration(format!(
                        "provider '{}' requires base_url", entry.name
                    ))
                })?;
                let mut p = OpenAiCompatibleProvider::new(entry.name.clone(), base, transport.clone());
                if let Ok(key_name) = std::env::var(format!("AISETU_PROVIDER_{}_API_KEY", entry.name.to_ascii_uppercase().replace('-', "_"))) {
                    if !key_name.is_empty() { p = p.with_api_key(key_name); }
                }
                providers.register(Arc::new(p));
            }
            other => {
                return Err(aisetu_core::SetuError::configuration(format!(
                    "unknown provider kind '{other}'"
                )));
            }
        }
    }

    let models = ModelRegistry::from_config(&config);
    let router = Router::new(models, providers);
    let shutdown = Shutdown::new();
    Ok(AppState::new(config, router, sessions, shutdown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_default_state() {
        let cfg = AppConfig::load(None).unwrap();
        let state = build_state(cfg).unwrap();
        assert!(state.router.resolve("aisetu-default").is_ok());
        assert!(state.router.resolve("aisetu-echo").is_ok());
    }
}
