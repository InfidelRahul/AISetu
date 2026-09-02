//! Canonical conversation: an ordered list of messages.

use serde::{Deserialize, Serialize};

use crate::{message::Message, role::Role, validate};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Conversation {
    pub messages: Vec<Message>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn with_messages(messages: Vec<Message>) -> Self {
        Self { messages }
    }

    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn system_messages(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter().filter(|m| m.role == Role::System)
    }

    pub fn last_user(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role == Role::User)
    }

    pub fn last_assistant(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn validate(&self) -> aisetu_core::Result<()> {
        validate::validate_conversation(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_query() {
        let mut c = Conversation::new();
        c.push(Message::system("sys"));
        c.push(Message::user("hello"));
        c.push(Message::assistant("hi"));
        assert_eq!(c.len(), 3);
        assert_eq!(c.last_user().unwrap().content.as_str(), "hello");
        assert_eq!(c.system_messages().count(), 1);
        c.validate().unwrap();
    }

    #[test]
    fn serde_roundtrip() {
        let c = Conversation::with_messages(vec![
            Message::system("s"),
            Message::user("u"),
            Message::assistant("a"),
        ]);
        let json = serde_json::to_string_pretty(&c).unwrap();
        let back: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
