//! Cookie jar used by the transport layer.

use std::fmt;

use cookie::Cookie;

/// Simple in-memory cookie jar.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct CookieJar {
    cookies: Vec<(String, String)>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: Vec::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        if let Some((_, existing)) = self.cookies.iter_mut().find(|(n, _)| n == &name) {
            *existing = value;
        } else {
            self.cookies.push((name, value));
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.cookies
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn remove(&mut self, name: &str) {
        self.cookies.retain(|(n, _)| n != name);
    }

    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.cookies.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Format as a Cookie request header value.
    pub fn header_value(&self) -> Option<String> {
        if self.cookies.is_empty() {
            None
        } else {
            Some(
                self.cookies
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
    }

    /// Merge Set-Cookie header values into the jar.
    pub fn absorb_set_cookie(&mut self, set_cookie: &str) {
        if let Ok(c) = Cookie::parse(set_cookie.to_string()) {
            self.add(c.name().to_string(), c.value().to_string());
        }
    }

    pub fn merge(&mut self, other: &CookieJar) {
        for (k, v) in other.iter() {
            self.add(k, v);
        }
    }
}

impl fmt::Debug for CookieJar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.cookies.iter().map(|(n, _)| n.as_str()).collect();
        f.debug_struct("CookieJar")
            .field("names", &names)
            .field("count", &self.cookies.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_get_header() {
        let mut jar = CookieJar::new();
        jar.add("sid", "abc");
        jar.add("theme", "dark");
        assert_eq!(jar.get("sid"), Some("abc"));
        let h = jar.header_value().unwrap();
        assert!(h.contains("sid=abc"));
        assert!(h.contains("theme=dark"));
    }

    #[test]
    fn absorb_set_cookie() {
        let mut jar = CookieJar::new();
        jar.absorb_set_cookie("session=xyz; Path=/; HttpOnly");
        assert_eq!(jar.get("session"), Some("xyz"));
    }

    #[test]
    fn debug_hides_values() {
        let mut jar = CookieJar::new();
        jar.add("sid", "super-secret");
        let d = format!("{jar:?}");
        assert!(!d.contains("super-secret"));
        assert!(d.contains("sid"));
    }
}
