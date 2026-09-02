//! Timeout configuration for HTTP requests.

use std::time::Duration;

/// Per-request timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeout {
    pub total: Duration,
    pub connect: Duration,
}

impl Timeout {
    pub fn from_duration(total: Duration) -> Self {
        Self {
            total,
            connect: total.min(Duration::from_secs(10)),
        }
    }

    pub fn from_millis(ms: u64) -> Self {
        Self::from_duration(Duration::from_millis(ms))
    }

    pub fn none() -> Self {
        Self {
            total: Duration::from_secs(u64::MAX / 1000),
            connect: Duration::from_secs(10),
        }
    }
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(10),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_millis() {
        let t = Timeout::from_millis(1500);
        assert_eq!(t.total, Duration::from_millis(1500));
    }
}
