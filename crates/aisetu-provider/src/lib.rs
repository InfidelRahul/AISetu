//! Provider adapters, capabilities, routing, and error normalization.

pub mod adapter;
pub mod capabilities;
pub mod echo;
pub mod error;
pub mod http_json;
pub mod mock;
pub mod openai_compatible;
pub mod registry;
pub mod reliability;
pub mod routing;

pub use adapter::{Provider, ProviderId};
pub use capabilities::{Capability, ProviderCapabilities};
pub use echo::EchoProvider;
pub use error::{normalize_http_error, CanonicalProviderError};
pub use http_json::HttpJsonProvider;
pub use mock::MockProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
pub use registry::{ModelRecord, ModelRegistry, ProviderRegistry};
pub use routing::Router;
