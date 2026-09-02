//! Secret and session redaction helpers for logs and traces.

const REDACTED: &str = "[REDACTED]";

const SENSITIVE_KEYS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "api_key",
    "apikey",
    "token",
    "access_token",
    "refresh_token",
    "session",
    "session_id",
    "password",
    "secret",
    "credential",
    "bearer",
];

/// Returns true when a header or field name should never be logged in full.
pub fn is_sensitive_key(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_KEYS
        .iter()
        .any(|k| lower == *k || lower.contains(k))
}

/// Redact a header value when the name is sensitive.
pub fn redact_header(name: &str, value: &str) -> String {
    if is_sensitive_key(name) {
        REDACTED.to_string()
    } else {
        value.to_string()
    }
}

/// Replace likely secrets inside an arbitrary string.
pub fn redact_text(input: &str) -> String {
    let mut out = input.to_string();
    for needle in [
        "Bearer ",
        "bearer ",
        "sk-",
        "sess-",
        "session=",
        "token=",
        "password=",
    ] {
        if let Some(idx) = out.find(needle) {
            let start = idx + needle.len();
            let end = out[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == ',' || c == ';')
                .map(|i| start + i)
                .unwrap_or(out.len());
            out.replace_range(start..end, REDACTED);
        }
    }
    out
}

pub fn redacted() -> &'static str {
    REDACTED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sensitive_keys() {
        assert!(is_sensitive_key("Authorization"));
        assert!(is_sensitive_key("Set-Cookie"));
        assert!(is_sensitive_key("x-api-key"));
        assert!(!is_sensitive_key("content-type"));
        assert!(!is_sensitive_key("user-agent"));
    }

    #[test]
    fn redacts_bearer() {
        let s = redact_text("Authorization: Bearer super-secret-token rest");
        assert!(s.contains(REDACTED));
        assert!(!s.contains("super-secret-token"));
    }
}
