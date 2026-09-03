//! Configuration loading foundation.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{limits::ResourceLimits, SetuError};

/// Where configuration was loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    File(PathBuf),
    Defaults,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

impl From<ConfigError> for SetuError {
    fn from(value: ConfigError) -> Self {
        SetuError::configuration(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub format: LogFormat,
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: LogFormat::Pretty,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Optional API key required in `Authorization: Bearer`. Empty disables auth.
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_bind() -> String {
    "127.0.0.1".into()
}

fn default_port() -> u16 {
    8080
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: default_port(),
            api_key: None,
        }
    }
}

impl ServerConfig {
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_true")]
    pub tls_verify: bool,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,
    /// Permit disabling TLS certificate verification. This must be explicitly enabled.
    #[serde(default)]
    pub allow_insecure_tls: bool,
}

fn default_timeout() -> u64 {
    60_000
}

fn default_connect_timeout() -> u64 {
    10_000
}

fn default_true() -> bool {
    true
}

fn default_user_agent() -> String {
    format!("AISetu/{}", env!("CARGO_PKG_VERSION"))
}

fn default_max_redirects() -> usize {
    5
}

fn default_enabled() -> bool {
    true
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout(),
            connect_timeout_ms: default_connect_timeout(),
            tls_verify: true,
            user_agent: default_user_agent(),
            max_redirects: default_max_redirects(),
            allow_insecure_tls: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderEntry {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapping {
    /// Name exposed on `/v1/models` and accepted in chat completions.
    pub id: String,
    /// Provider name from `providers`.
    pub provider: String,
    /// Optional provider-native model identifier.
    #[serde(default)]
    pub upstream_model: Option<String>,
}

impl Default for ModelMapping {
    fn default() -> Self {
        Self {
            id: "aisetu-default".into(),
            provider: "mock".into(),
            upstream_model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    /// Directory for non-secret runtime state.
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
    /// Directory for permission-protected secret files; contents are not encrypted by this store.
    #[serde(default)]
    pub secrets_dir: Option<PathBuf>,
}

impl StorageConfig {
    pub fn resolve_data_dir(&self) -> PathBuf {
        self.data_dir
            .clone()
            .or_else(|| dirs::data_dir().map(|p| p.join("aisetu")))
            .unwrap_or_else(|| PathBuf::from("./data"))
    }

    pub fn resolve_secrets_dir(&self) -> PathBuf {
        self.secrets_dir
            .clone()
            .or_else(|| dirs::data_dir().map(|p| p.join("aisetu").join("secrets")))
            .unwrap_or_else(|| PathBuf::from("./secrets"))
    }
}

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub limits: ResourceLimits,
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
    #[serde(default)]
    pub models: Vec<ModelMapping>,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(skip)]
    pub source: Option<String>,
}

impl AppConfig {
    /// Load configuration from an optional path, falling back to defaults.
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let resolved = match path {
            Some(p) => Some(p.to_path_buf()),
            None => discover_config_path(),
        };

        let mut cfg = match resolved {
            Some(p) if p.exists() => Self::from_file(&p)?,
            Some(p) => {
                return Err(ConfigError::Invalid(format!(
                    "config file not found: {}",
                    p.display()
                )))
            }
            None => Self::default(),
        };

        cfg.apply_env_overrides();
        cfg.ensure_defaults();
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let raw = fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut cfg: Self = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        cfg.source = Some(path.display().to_string());
        Ok(cfg)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        let mut cfg: Self = toml::from_str(raw).map_err(|source| ConfigError::Parse {
            path: PathBuf::from("<memory>"),
            source,
        })?;
        cfg.ensure_defaults();
        cfg.validate()?;
        Ok(cfg)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = env::var("AISETU_BIND") {
            self.server.bind = v;
        }
        if let Ok(v) = env::var("AISETU_PORT") {
            if let Ok(p) = v.parse() {
                self.server.port = p;
            }
        }
        if let Ok(v) = env::var("AISETU_API_KEY") {
            if !v.is_empty() {
                self.server.api_key = Some(v);
            }
        }
        if let Ok(v) = env::var("AISETU_LOG_LEVEL") {
            self.log.level = v;
        }
        if let Ok(v) = env::var("AISETU_LOG_FORMAT") {
            if v.eq_ignore_ascii_case("json") {
                self.log.format = LogFormat::Json;
            } else {
                self.log.format = LogFormat::Pretty;
            }
        }
    }

    fn ensure_defaults(&mut self) {
        if self.providers.is_empty() {
            self.providers.push(ProviderEntry {
                name: "mock".into(),
                kind: "mock".into(),
                base_url: None,
                model: Some("mock-text".into()),
                enabled: true,
            });
            self.providers.push(ProviderEntry {
                name: "echo".into(),
                kind: "echo".into(),
                base_url: None,
                model: Some("echo-text".into()),
                enabled: true,
            });
        }
        if self.models.is_empty() {
            self.models.push(ModelMapping {
                id: "aisetu-default".into(),
                provider: "mock".into(),
                upstream_model: Some("mock-text".into()),
            });
            self.models.push(ModelMapping {
                id: "aisetu-echo".into(),
                provider: "echo".into(),
                upstream_model: Some("echo-text".into()),
            });
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.limits.validate().map_err(ConfigError::Invalid)?;
        if self.server.port == 0 {
            return Err(ConfigError::Invalid("server.port must be > 0".into()));
        }
        if self.server.bind.trim().is_empty() {
            return Err(ConfigError::Invalid("server.bind must not be empty".into()));
        }
        if self.transport.timeout_ms == 0 {
            return Err(ConfigError::Invalid(
                "transport.timeout_ms must be > 0".into(),
            ));
        }
        for p in &self.providers {
            if p.name.trim().is_empty() {
                return Err(ConfigError::Invalid(
                    "provider name must not be empty".into(),
                ));
            }
            if p.kind.trim().is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "provider '{}' kind must not be empty",
                    p.name
                )));
            }
        }
        for m in &self.models {
            if m.id.trim().is_empty() {
                return Err(ConfigError::Invalid("model id must not be empty".into()));
            }
            if !self.providers.iter().any(|p| p.name == m.provider) {
                return Err(ConfigError::Invalid(format!(
                    "model '{}' references unknown provider '{}'",
                    m.id, m.provider
                )));
            }
        }
        Ok(())
    
        if self.transport.connect_timeout_ms == 0 {
            return Err(ConfigError::Invalid("connect_timeout_ms must be greater than zero".into()));
        }
        if !self.transport.tls_verify && !self.transport.allow_insecure_tls {
            return Err(ConfigError::Invalid("tls_verify=false requires allow_insecure_tls=true".into()));
        }
        if !is_loopback_bind(&self.server.bind) && self.server.api_key.as_deref().unwrap_or("").is_empty() {
            return Err(ConfigError::Invalid("api_key is required when binding to a non-loopback address".into()));
        }
        let supported = ["mock", "echo", "http", "http_json"];
        let mut providers = std::collections::HashSet::new();
        for provider in &self.providers {
            if provider.name.trim().is_empty() {
                return Err(ConfigError::Invalid("provider name must not be empty".into()));
            }
            if !supported.contains(&provider.kind.as_str()) {
                return Err(ConfigError::Invalid(format!("unsupported provider kind: {}", provider.kind)));
            }
            if !providers.insert(provider.name.as_str()) {
                return Err(ConfigError::Invalid(format!("duplicate provider: {}", provider.name)));
            }
            if provider.enabled && matches!(provider.kind.as_str(), "http" | "http_json") {
                let raw = provider.base_url.as_deref().ok_or_else(|| ConfigError::Invalid(format!("provider {} requires base_url", provider.name)))?;
                let parsed = url::Url::parse(raw).map_err(|_| ConfigError::Invalid(format!("invalid base_url for provider {}", provider.name)))?;
                if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
                    return Err(ConfigError::Invalid(format!("base_url for provider {} must use http/https and include a host", provider.name)));
                }
            }
        }
        let mut models = std::collections::HashSet::new();
        for model in &self.models {
            if !models.insert(model.id.as_str()) {
                return Err(ConfigError::Invalid(format!("duplicate model id: {}", model.id)));
            }
            if !providers.contains(model.provider.as_str()) {
                return Err(ConfigError::Invalid(format!("model {} references unknown provider {}", model.id, model.provider)));
            }
            if let Some(provider) = self.providers.iter().find(|p| p.name == model.provider) {
                if !provider.enabled {
                    return Err(ConfigError::Invalid(format!("model {} references disabled provider {}", model.id, model.provider)));
                }
            }
        }
        Ok(())
    }

    pub fn provider(&self, name: &str) -> Option<&ProviderEntry> {
        self.providers.iter().find(|p| p.name == name)
    }

    pub fn model(&self, id: &str) -> Option<&ModelMapping> {
        self.models.iter().find(|m| m.id == id)
    }
}

fn discover_config_path() -> Option<PathBuf> {
    if let Ok(p) = env::var("AISETU_CONFIG") {
        return Some(PathBuf::from(p));
    }
    let mut candidates = vec![
        PathBuf::from("aisetu.toml"),
        PathBuf::from("config/aisetu.toml"),
    ];
    if let Some(p) = dirs::config_dir() {
        candidates.push(p.join("aisetu").join("aisetu.toml"));
    }
    candidates.into_iter().find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_validates() {
        let cfg = AppConfig::load(None).unwrap();
        cfg.validate().unwrap();
        assert!(!cfg.providers.is_empty());
        assert!(!cfg.models.is_empty());
        assert!(cfg.model("aisetu-default").is_some());
    }

    #[test]
    fn parse_toml() {
        let raw = r#"
            [server]
            bind = "0.0.0.0"
            port = 9090

            [log]
            level = "debug"
            format = "json"

            [[providers]]
            name = "mock"
            kind = "mock"
            enabled = true

            [[models]]
            id = "gpt-aisetu"
            provider = "mock"
        "#;
        let cfg = AppConfig::from_toml_str(raw).unwrap();
        assert_eq!(cfg.server.port, 9090);
        assert_eq!(cfg.server.bind, "0.0.0.0");
        assert_eq!(cfg.log.level, "debug");
        assert_eq!(cfg.models[0].id, "gpt-aisetu");
    }

    #[test]
    fn unknown_provider_rejected() {
        let raw = r#"
            [[providers]]
            name = "mock"
            kind = "mock"

            [[models]]
            id = "x"
            provider = "missing"
        "#;
        assert!(AppConfig::from_toml_str(raw).is_err());
    }

    #[test]
    fn roundtrip_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("aisetu.toml");
        fs::write(
            &path,
            r#"
            [server]
            port = 8111
            [[providers]]
            name = "mock"
            kind = "mock"
            enabled = true
            [[models]]
            id = "m"
            provider = "mock"
            "#,
        )
        .unwrap();
        let cfg = AppConfig::load(Some(&path)).unwrap();
        assert_eq!(cfg.server.port, 8111);
    }
}


fn is_loopback_bind(bind: &str) -> bool {
    bind.eq_ignore_ascii_case("localhost")
        || bind.parse::<std::net::IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false)
}
