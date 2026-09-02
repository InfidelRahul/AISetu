//! Extract assistant text from provider representations.

use serde_json::Value;

/// Extract a text field from a provider payload.
pub trait Extractor: Send + Sync {
    fn extract_text(&self, payload: &Value) -> aisetu_core::Result<String>;
}

/// Extract via a simple dotted JSON path, e.g. `choices.0.message.content`.
pub struct JsonPathExtractor {
    pub path: String,
}

impl JsonPathExtractor {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Extractor for JsonPathExtractor {
    fn extract_text(&self, payload: &Value) -> aisetu_core::Result<String> {
        let mut current = payload;
        for segment in self.path.split('.') {
            if segment.is_empty() {
                continue;
            }
            current = if let Ok(idx) = segment.parse::<usize>() {
                current.get(idx).ok_or_else(|| {
                    aisetu_core::SetuError::parse_failure(format!(
                        "path '{}' missing index {idx}",
                        self.path
                    ))
                })?
            } else {
                current.get(segment).ok_or_else(|| {
                    aisetu_core::SetuError::parse_failure(format!(
                        "path '{}' missing field '{segment}'",
                        self.path
                    ))
                })?
            };
        }
        match current {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            other => Err(aisetu_core::SetuError::parse_failure(format!(
                "path '{}' did not resolve to text, got {other}",
                self.path
            ))),
        }
    }
}

/// Extract first capture group of a regex-like simple pattern `prefix(.*?)suffix`.
pub struct RegexExtractor {
    pub prefix: String,
    pub suffix: String,
}

impl RegexExtractor {
    pub fn between(prefix: impl Into<String>, suffix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            suffix: suffix.into(),
        }
    }
}

impl Extractor for RegexExtractor {
    fn extract_text(&self, payload: &Value) -> aisetu_core::Result<String> {
        let raw = match payload {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let start = raw.find(&self.prefix).ok_or_else(|| {
            aisetu_core::SetuError::parse_failure(format!(
                "prefix '{}' not found in provider payload",
                self.prefix
            ))
        })?;
        let from = start + self.prefix.len();
        let end = raw[from..]
            .find(&self.suffix)
            .map(|i| from + i)
            .ok_or_else(|| {
                aisetu_core::SetuError::parse_failure(format!(
                    "suffix '{}' not found in provider payload",
                    self.suffix
                ))
            })?;
        Ok(raw[from..end].trim().to_string())
    }
}

/// Try a sequence of extractors until one succeeds.
pub struct FallbackExtractor {
    pub extractors: Vec<Box<dyn Extractor>>,
}

impl FallbackExtractor {
    pub fn new(extractors: Vec<Box<dyn Extractor>>) -> Self {
        Self { extractors }
    }
}

impl Extractor for FallbackExtractor {
    fn extract_text(&self, payload: &Value) -> aisetu_core::Result<String> {
        let mut last = aisetu_core::SetuError::parse_failure("no extractors configured");
        for ex in &self.extractors {
            match ex.extract_text(payload) {
                Ok(text) if !text.trim().is_empty() => return Ok(text),
                Ok(_) => {
                    last = aisetu_core::SetuError::parse_failure("extractor returned empty text");
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_path() {
        let payload = json!({"choices":[{"message":{"content":"hello"}}]});
        let ex = JsonPathExtractor::new("choices.0.message.content");
        assert_eq!(ex.extract_text(&payload).unwrap(), "hello");
    }

    #[test]
    fn json_path_missing() {
        let payload = json!({"choices":[]});
        let ex = JsonPathExtractor::new("choices.0.message.content");
        assert!(ex.extract_text(&payload).is_err());
    }

    #[test]
    fn regex_between() {
        let payload = json!("PRE hello SUF");
        let ex = RegexExtractor::between("PRE ", " SUF");
        assert_eq!(ex.extract_text(&payload).unwrap(), "hello");
    }

    #[test]
    fn fallback() {
        let payload = json!({"text":"ok"});
        let fb = FallbackExtractor::new(vec![
            Box::new(JsonPathExtractor::new("missing")),
            Box::new(JsonPathExtractor::new("text")),
        ]);
        assert_eq!(fb.extract_text(&payload).unwrap(), "ok");
    }
}
