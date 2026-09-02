//! HTTP header map with redaction-aware debug.

use std::fmt;

use aisetu_core::redact;

/// Case-insensitive header map that preserves insertion order of names.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct HeaderMap {
    inner: Vec<(String, String)>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        if let Some((_, existing)) = self
            .inner
            .iter_mut()
            .find(|(n, _)| n.eq_ignore_ascii_case(&name))
        {
            *existing = value;
        } else {
            self.inner.push((name, value));
        }
    }

    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.inner.push((name.into(), value.into()));
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.inner
            .iter()
            .filter(move |(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn remove(&mut self, name: &str) {
        self.inner.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn content_type(&self) -> Option<&str> {
        self.get("content-type")
    }

    pub fn set_content_type(&mut self, value: impl Into<String>) {
        self.insert("content-type", value);
    }
}

impl fmt::Debug for HeaderMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (k, v) in &self.inner {
            map.entry(k, &redact::redact_header(k, v));
        }
        map.finish()
    }
}

impl FromIterator<(String, String)> for HeaderMap {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        let mut h = HeaderMap::new();
        for (k, v) in iter {
            h.append(k, v);
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_is_case_insensitive() {
        let mut h = HeaderMap::new();
        h.insert("Content-Type", "text/plain");
        h.insert("content-type", "application/json");
        assert_eq!(h.get("CONTENT-TYPE"), Some("application/json"));
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn debug_redacts_authorization() {
        let mut h = HeaderMap::new();
        h.insert("Authorization", "Bearer secret");
        let d = format!("{h:?}");
        assert!(!d.contains("secret"));
        assert!(d.contains("REDACTED"));
    }
}
