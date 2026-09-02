//! Canonical internal conversation language for AISetu.
//!
//! Conversation, Message, Role, and text content. Provider-independent.

pub mod content;
pub mod conversation;
pub mod message;
pub mod request;
pub mod response;
pub mod role;
pub mod validate;

pub use content::TextContent;
pub use conversation::Conversation;
pub use message::Message;
pub use request::ConversationRequest;
pub use response::{ConversationResponse, FinishReason, Usage};
pub use role::Role;
pub use validate::validate_conversation;
