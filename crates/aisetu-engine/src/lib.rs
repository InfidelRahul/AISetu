//! Provider-independent conversation translation framework.
//!
//! Canonical Conversation
//!         ↕
//! Translation Engine
//!         ↕
//! Provider Representation

pub mod extractor;
pub mod normalizer;
pub mod translator;
pub mod validator;

pub use extractor::{Extractor, JsonPathExtractor, RegexExtractor};
pub use normalizer::{Normalizer, WhitespaceNormalizer};
pub use translator::{
    ConversationTranslator, ProviderRepresentation, RequestTranslator, ResponseTranslator,
    TranslationContext,
};
pub use validator::{SchemaValidator, Validator};
