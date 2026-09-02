//! Browser plugin trait and two replaceable implementations.

use std::process::Command;

use async_trait::async_trait;
use url::Url;

use crate::capture::{CapturedSession, SessionCapture};

/// Launch a browser, complete login, capture permitted session state.
#[async_trait]
pub trait BrowserPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    async fn authenticate(
        &self,
        login_url: &str,
        provider: &str,
        capture: &SessionCapture,
    ) -> aisetu_core::Result<CapturedSession>;
}

/// Opens the system browser at the login URL. Session cookies must be supplied
/// out of band (environment / callback). Used as the default interactive path.
pub struct SystemBrowser;

#[async_trait]
impl BrowserPlugin for SystemBrowser {
    fn name(&self) -> &'static str {
        "system"
    }

    async fn authenticate(
        &self,
        login_url: &str,
        provider: &str,
        capture: &SessionCapture,
    ) -> aisetu_core::Result<CapturedSession> {
        let url = Url::parse(login_url).map_err(|e| {
            aisetu_core::SetuError::invalid_request(format!("invalid login url: {e}"))
        })?;
        tracing::info!(url = %url, provider, "launching system browser for login");
        let _ = open_system(url.as_str());
        let mut captured = CapturedSession::new(provider);
        if let Ok(cookie) = std::env::var("AISETU_SESSION_COOKIE") {
            if let Some((k, v)) = cookie.split_once('=') {
                captured.cookies.insert(k.to_string(), v.to_string());
            }
        }
        Ok(capture.filter(captured))
    }
}

fn open_system(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).status()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", url]).status()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).status()?;
    }
    Ok(())
}

type AuthScript = dyn Fn(&str, &str) -> aisetu_core::Result<CapturedSession> + Send + Sync;

/// Headless / scripted browser used in tests and automated bootstrap.
///
/// Does not parse provider conversations. It only returns a captured session
/// produced by the supplied script.
pub struct HeadlessScriptBrowser {
    script: Box<AuthScript>,
}

impl HeadlessScriptBrowser {
    pub fn new(
        script: impl Fn(&str, &str) -> aisetu_core::Result<CapturedSession> + Send + Sync + 'static,
    ) -> Self {
        Self {
            script: Box::new(script),
        }
    }

    /// Test helper that pretends a login succeeded with a single cookie.
    pub fn succeeding(cookie_name: &'static str, cookie_value: &'static str) -> Self {
        Self::new(move |_url, provider| {
            let mut cap = CapturedSession::new(provider);
            cap.cookies
                .insert(cookie_name.to_string(), cookie_value.to_string());
            Ok(cap)
        })
    }
}

#[async_trait]
impl BrowserPlugin for HeadlessScriptBrowser {
    fn name(&self) -> &'static str {
        "headless-script"
    }

    async fn authenticate(
        &self,
        login_url: &str,
        provider: &str,
        capture: &SessionCapture,
    ) -> aisetu_core::Result<CapturedSession> {
        let _ = Url::parse(login_url).map_err(|e| {
            aisetu_core::SetuError::invalid_request(format!("invalid login url: {e}"))
        })?;
        let captured = (self.script)(login_url, provider)?;
        Ok(capture.filter(captured))
    }
}

/// Transfer a captured session into the session manager.
pub async fn transfer_to_manager(
    manager: &aisetu_session::SessionManager,
    captured: CapturedSession,
) -> aisetu_core::Result<aisetu_session::Session> {
    let session = captured.into_session();
    manager.create(session).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use aisetu_session::{MemorySessionStore, SessionManager};
    use std::sync::Arc;

    #[tokio::test]
    async fn headless_login_transfers() {
        let browser = HeadlessScriptBrowser::succeeding("sid", "abc123");
        let captured = browser
            .authenticate(
                "https://example.com/login",
                "web",
                &SessionCapture::allow_cookies(&["sid"]),
            )
            .await
            .unwrap();
        assert_eq!(
            captured.cookies.get("sid").map(String::as_str),
            Some("abc123")
        );

        let mgr = SessionManager::new(Arc::new(MemorySessionStore::new()));
        let session = transfer_to_manager(&mgr, captured).await.unwrap();
        assert_eq!(session.provider, "web");
        assert_eq!(
            session.cookies.get("sid").map(String::as_str),
            Some("abc123")
        );
    }

    #[tokio::test]
    async fn rejects_bad_url() {
        let browser = HeadlessScriptBrowser::succeeding("sid", "x");
        let err = browser
            .authenticate("not-a-url", "web", &SessionCapture::permissive())
            .await
            .unwrap_err();
        assert_eq!(err.kind, aisetu_core::ErrorKind::InvalidRequest);
    }
}
