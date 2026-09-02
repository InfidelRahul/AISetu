//! Conversation validation rules.

use aisetu_core::SetuError;

use crate::{conversation::Conversation, role::Role};

/// Validate a canonical conversation.
///
/// Rules:
/// - at least one message
/// - at least one user message
/// - no empty content
/// - first non-system message must be user
/// - system messages may only appear as a prefix (or be absent)
pub fn validate_conversation(conversation: &Conversation) -> aisetu_core::Result<()> {
    if conversation.messages.is_empty() {
        return Err(SetuError::validation("conversation has no messages"));
    }

    let mut seen_non_system = false;
    let mut has_user = false;
    for (idx, msg) in conversation.messages.iter().enumerate() {
        if msg.is_empty() {
            return Err(SetuError::validation(format!(
                "message {idx} has empty content"
            )));
        }
        match msg.role {
            Role::System => {
                if seen_non_system {
                    return Err(SetuError::validation(format!(
                        "system message at index {idx} appears after a non-system message"
                    )));
                }
            }
            Role::User => {
                seen_non_system = true;
                has_user = true;
            }
            Role::Assistant => {
                if !seen_non_system {
                    return Err(SetuError::validation(
                        "conversation cannot start with an assistant message",
                    ));
                }
                seen_non_system = true;
            }
        }
    }

    if !has_user {
        return Err(SetuError::validation(
            "conversation must contain at least one user message",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    #[test]
    fn empty_rejected() {
        assert!(validate_conversation(&Conversation::new()).is_err());
    }

    #[test]
    fn system_only_rejected() {
        let c = Conversation::with_messages(vec![Message::system("s")]);
        assert!(validate_conversation(&c).is_err());
    }

    #[test]
    fn assistant_first_rejected() {
        let c = Conversation::with_messages(vec![Message::assistant("a")]);
        assert!(validate_conversation(&c).is_err());
    }

    #[test]
    fn system_after_user_rejected() {
        let c = Conversation::with_messages(vec![Message::user("u"), Message::system("late")]);
        assert!(validate_conversation(&c).is_err());
    }

    #[test]
    fn empty_content_rejected() {
        let c = Conversation::with_messages(vec![Message::user("   ")]);
        assert!(validate_conversation(&c).is_err());
    }

    #[test]
    fn valid_shapes() {
        let a = Conversation::with_messages(vec![Message::user("hi")]);
        validate_conversation(&a).unwrap();
        let b = Conversation::with_messages(vec![
            Message::system("s"),
            Message::user("u"),
            Message::assistant("a"),
            Message::user("u2"),
        ]);
        validate_conversation(&b).unwrap();
    }
}
