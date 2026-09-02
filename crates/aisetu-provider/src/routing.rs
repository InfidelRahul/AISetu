//! Model → provider routing.

use std::sync::Arc;

use crate::{
    adapter::Provider,
    capabilities::{Capability, ProviderCapabilities},
    registry::{ModelRecord, ModelRegistry, ProviderRegistry},
};

pub struct Router {
    pub models: ModelRegistry,
    pub providers: ProviderRegistry,
}

impl Router {
    pub fn new(models: ModelRegistry, providers: ProviderRegistry) -> Self {
        Self { models, providers }
    }

    /// Resolve a requested model id to (record, provider).
    pub fn resolve(&self, model: &str) -> aisetu_core::Result<(ModelRecord, Arc<dyn Provider>)> {
        let record =
            self.models.get(model).cloned().ok_or_else(|| {
                aisetu_core::SetuError::not_found(format!("unknown model '{model}'"))
            })?;
        let provider = self.providers.get(&record.provider).ok_or_else(|| {
            aisetu_core::SetuError::configuration(format!(
                "model '{model}' maps to unknown provider '{}'",
                record.provider
            ))
        })?;
        Ok((record, provider))
    }

    pub fn require(
        &self,
        model: &str,
        cap: Capability,
    ) -> aisetu_core::Result<(ModelRecord, Arc<dyn Provider>)> {
        let (record, provider) = self.resolve(model)?;
        provider.capabilities().require(cap)?;
        Ok((record, provider))
    }

    pub fn capabilities_for_model(&self, model: &str) -> aisetu_core::Result<ProviderCapabilities> {
        let (_, provider) = self.resolve(model)?;
        Ok(provider.capabilities().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EchoProvider, MockProvider};

    #[test]
    fn routes_by_model() {
        let mut providers = ProviderRegistry::new();
        providers.register(Arc::new(MockProvider::new("mock")));
        providers.register(Arc::new(EchoProvider::new("echo")));
        let models = ModelRegistry::from_records(vec![
            ModelRecord {
                id: "aisetu-default".into(),
                provider: "mock".into(),
                upstream_model: None,
                owned_by: "aisetu".into(),
            },
            ModelRecord {
                id: "aisetu-echo".into(),
                provider: "echo".into(),
                upstream_model: None,
                owned_by: "aisetu".into(),
            },
        ]);
        let router = Router::new(models, providers);
        let (rec, p) = router.resolve("aisetu-echo").unwrap();
        assert_eq!(rec.provider, "echo");
        assert_eq!(p.id().as_str(), "echo");
        assert!(router.resolve("nope").is_err());
    }
}
