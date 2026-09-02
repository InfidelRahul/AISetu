//! Lightweight, replaceable browser plugin for session bootstrap.
//!
//! Browser → Session Manager → HTTP Layer
//!
//! The browser is intentionally independent from conversation processing.

pub mod capture;
pub mod plugin;

pub use capture::{CapturedSession, SessionCapture};
pub use plugin::{transfer_to_manager, BrowserPlugin, HeadlessScriptBrowser, SystemBrowser};
