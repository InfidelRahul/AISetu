//! Provider and model registries.

use std::collections::HashMap;
use std::sync::Arc;

use aisetu_core::config::{AppConfig, ModelMapping};

use crate::{
    adapter::{Provider, ProviderId},
    capabilities::ProviderCapabilities,
};

#[derive(Debug, Clone)]
pub struct ModelRecord {
    pub id: String,
    pub provider: String,
    pub upstream_model: Option<String>,
    pub owned_by: String,
}

impl From<&ModelMapping> for ModelRecord {
    fn from(m: &ModelMapping) -> Self {
        Self {
            id: m.id.clone(),
            provider: m.provider.clone(),
            upstream_model: m.upstream_model.clone(),
            owned_by: "aisetu".into(),
        }
    }
}

#[derive(Clone, Default)]
pub struct ModelRegistry {
    models: Vec<ModelRecord>,
}

impl ModelRegistry {
    pub fn from_config(cfg: &AppConfig) -> Self {
        Self {
            models: cfg.models.iter().map(ModelRecord::from).collect(),
        }
    }

    pub fn from_records(models: Vec<ModelRecord>) -> Self {
        Self { models }
    }

    pub fn list(&self) -> &[ModelRecord] {
        &self.models
    }

    pub fn get(&self, id: &str) -> Option<&ModelRecord> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn default_model(&self) -> Option<&ModelRecord> {
        self.models.first()
    }
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        let name = provider.id().as_str().to_string();
        self.providers.insert(name, provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(name).cloned()
    }

    pub fn get_id(&self, id: &ProviderId) -> Option<Arc<dyn Provider>> {
        self.get(id.as_str())
    }

    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn capabilities(&self, name: &str) -> Option<ProviderCapabilities> {
        self.get(name).map(|p| p.capabilities().clone())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;

    #[test]
    fn register_and_get() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(MockProvider::new("mock")));
        assert!(reg.get("mock").is_some());
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn models_from_config() {
        let cfg = aisetu_core::AppConfig::load(None).unwrap();
        let models = ModelRegistry::from_config(&cfg);
        assert!(models.get("aisetu-default").is_some());
    }
}
