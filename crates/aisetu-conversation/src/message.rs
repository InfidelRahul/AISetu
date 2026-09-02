//! Canonical conversation message.

use serde::{Deserialize, Serialize};

use crate::{role::Role, TextContent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: TextContent,
}

impl Message {
    pub fn new(role: Role, content: impl Into<TextContent>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, TextContent::new(text))
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, TextContent::new(text))
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, TextContent::new(text))
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors() {
        let m = Message::user("hi");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content.as_str(), "hi");
    }

    #[test]
    fn serde_roundtrip() {
        let m = Message::system("be brief");
        let json = serde_json::to_string(&m).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
