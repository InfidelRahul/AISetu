//! Intelligence abstraction and Needle implementation.
//!
//! input + schema + context → structured result + confidence

pub mod engine;
pub mod needle;
pub mod schema;
pub mod types;

pub use engine::{DeterministicEngine, IntelligenceEngine};
pub use needle::NeedleEngine;
pub use schema::JsonSchema;
pub use types::{IntelligenceContext, IntelligenceInput, IntelligenceOutput};
