//! Generic HTTP transport.
//!
//! Transport
//!    ↓
//! Request
//!    ↓
//! Response
//!
//! This crate does not know about providers, conversations, or APIs.

pub mod client;
pub mod cookie_jar;
pub mod headers;
pub mod request;
pub mod response;
pub mod timeout;

pub use client::{HttpTransport, Transport};
pub use cookie_jar::CookieJar;
pub use headers::HeaderMap;
pub use request::{Body, HttpRequest, Method};
pub use response::{HttpResponse, StatusCode};
pub use timeout::Timeout;
