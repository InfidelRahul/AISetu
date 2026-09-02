//! Text content of a message.

use serde::{Deserialize, Serialize};

/// Canonical text payload. Multimodal content is intentionally out of scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl From<String> for TextContent {
    fn from(value: String) -> Self {
        Self { text: value }
    }
}

impl From<&str> for TextContent {
    fn from(value: &str) -> Self {
        Self {
            text: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_detection() {
        assert!(TextContent::new("  ").is_empty());
        assert!(!TextContent::new("hello").is_empty());
    }
}
