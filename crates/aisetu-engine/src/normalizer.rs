//! Normalize provider-specific payloads toward a stable shape.

use aisetu_conversation::{Conversation, Message, Role};

/// Normalize inbound or outbound representations.
pub trait Normalizer: Send + Sync {
    fn normalize_text(&self, input: &str) -> String;
    fn normalize_conversation(&self, conversation: &Conversation) -> Conversation {
        Conversation::with_messages(
            conversation
                .messages
                .iter()
                .map(|m| Message::new(m.role, self.normalize_text(m.content.as_str())))
                .collect(),
        )
    }
}

/// Trim, collapse interior whitespace, and drop empty messages.
pub struct WhitespaceNormalizer;

impl Normalizer for WhitespaceNormalizer {
    fn normalize_text(&self, input: &str) -> String {
        let collapsed: String = input.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed
    }

    fn normalize_conversation(&self, conversation: &Conversation) -> Conversation {
        let mut out = Vec::new();
        let mut last_role: Option<Role> = None;
        for msg in &conversation.messages {
            let text = self.normalize_text(msg.content.as_str());
            if text.is_empty() {
                continue;
            }
            if last_role == Some(msg.role) && msg.role != Role::System {
                if let Some(prev) = out.last_mut() {
                    let Message { content, .. } = prev;
                    content.text = format!("{} {}", content.text, text);
                    continue;
                }
            }
            last_role = Some(msg.role);
            out.push(Message::new(msg.role, text));
        }
        Conversation::with_messages(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace() {
        let n = WhitespaceNormalizer;
        assert_eq!(n.normalize_text("  hello   world \n"), "hello world");
    }

    #[test]
    fn merges_adjacent_same_role() {
        let n = WhitespaceNormalizer;
        let c = Conversation::with_messages(vec![
            Message::user("hello"),
            Message::user("world"),
            Message::assistant("ok"),
        ]);
        let out = n.normalize_conversation(&c);
        assert_eq!(out.len(), 2);
        assert_eq!(out.messages[0].content.as_str(), "hello world");
    }
}
