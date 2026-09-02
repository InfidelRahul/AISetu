//! Session persistence.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::session::{Session, SessionId};

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn save(&self, session: &Session) -> aisetu_core::Result<()>;
    async fn load(&self, id: &SessionId) -> aisetu_core::Result<Option<Session>>;
    async fn load_for_provider(&self, provider: &str) -> aisetu_core::Result<Option<Session>>;
    async fn delete(&self, id: &SessionId) -> aisetu_core::Result<()>;
    async fn list(&self) -> aisetu_core::Result<Vec<SessionId>>;
}

#[derive(Default)]
pub struct MemorySessionStore {
    inner: RwLock<Vec<Session>>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl SessionStore for MemorySessionStore {
    async fn save(&self, session: &Session) -> aisetu_core::Result<()> {
        let mut guard = self.inner.write();
        if let Some(existing) = guard
            .iter_mut()
            .find(|s| s.id.as_str() == session.id.as_str())
        {
            *existing = session.clone();
        } else {
            guard.push(session.clone());
        }
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> aisetu_core::Result<Option<Session>> {
        Ok(self
            .inner
            .read()
            .iter()
            .find(|s| s.id.as_str() == id.as_str())
            .cloned())
    }

    async fn load_for_provider(&self, provider: &str) -> aisetu_core::Result<Option<Session>> {
        Ok(self
            .inner
            .read()
            .iter()
            .filter(|s| s.provider == provider)
            .max_by_key(|s| s.updated_at)
            .cloned())
    }

    async fn delete(&self, id: &SessionId) -> aisetu_core::Result<()> {
        self.inner.write().retain(|s| s.id.as_str() != id.as_str());
        Ok(())
    }

    async fn list(&self) -> aisetu_core::Result<Vec<SessionId>> {
        Ok(self.inner.read().iter().map(|s| s.id.clone()).collect())
    }
}

/// File-backed store. Files live under a dedicated secrets directory.
pub struct FileSessionStore {
    dir: PathBuf,
}

impl FileSessionStore {
    pub fn new(dir: impl Into<PathBuf>) -> aisetu_core::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).map_err(|e| {
            aisetu_core::SetuError::configuration(format!(
                "failed to create session dir {}: {e}",
                dir.display()
            ))
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            let _ = std::fs::set_permissions(&dir, perms);
        }
        Ok(Self { dir })
    }

    fn path_for(&self, id: &SessionId) -> PathBuf {
        self.dir.join(format!("{}.json", id.as_str()))
    }

    fn read_file(&self, path: &Path) -> aisetu_core::Result<Session> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| aisetu_core::SetuError::internal(format!("read session: {e}")))?;
        serde_json::from_str(&raw).map_err(|e| {
            aisetu_core::SetuError::parse_failure(format!("corrupt session file: {e}"))
        })
    }
}

#[async_trait]
impl SessionStore for FileSessionStore {
    async fn save(&self, session: &Session) -> aisetu_core::Result<()> {
        let path = self.path_for(&session.id);
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(session)
            .map_err(|e| aisetu_core::SetuError::internal(format!("serialize session: {e}")))?;
        std::fs::write(&tmp, json)
            .map_err(|e| aisetu_core::SetuError::internal(format!("write session: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, &path)
            .map_err(|e| aisetu_core::SetuError::internal(format!("commit session: {e}")))?;
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> aisetu_core::Result<Option<Session>> {
        let path = self.path_for(id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(self.read_file(&path)?))
    }

    async fn load_for_provider(&self, provider: &str) -> aisetu_core::Result<Option<Session>> {
        let mut best: Option<Session> = None;
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| aisetu_core::SetuError::internal(format!("list sessions: {e}")))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(session) = self.read_file(&path) {
                if session.provider == provider {
                    match &best {
                        Some(current) if current.updated_at >= session.updated_at => {}
                        _ => best = Some(session),
                    }
                }
            }
        }
        Ok(best)
    }

    async fn delete(&self, id: &SessionId) -> aisetu_core::Result<()> {
        let path = self.path_for(id);
        if path.exists() {
            let cleared = Session {
                id: id.clone(),
                provider: String::new(),
                cookies: Default::default(),
                headers: Default::default(),
                state: crate::SessionState::Invalid,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                expires_at: None,
            };
            let _ = serde_json::to_string(&cleared);
            std::fs::write(&path, b"{}").ok();
            std::fs::remove_file(&path)
                .map_err(|e| aisetu_core::SetuError::internal(format!("delete session: {e}")))?;
        }
        Ok(())
    }

    async fn list(&self) -> aisetu_core::Result<Vec<SessionId>> {
        let mut ids = Vec::new();
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(_) => return Ok(ids),
        };
        for entry in entries.flatten() {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                if stem.starts_with("sess_") {
                    ids.push(SessionId::from_raw(stem));
                }
            }
        }
        Ok(ids)
    }
}

pub fn shared_memory() -> Arc<dyn SessionStore> {
    Arc::new(MemorySessionStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;

    #[tokio::test]
    async fn memory_crud() {
        let store = MemorySessionStore::new();
        let s = Session::new("mock").cookie("sid", "abc");
        let id = s.id.clone();
        store.save(&s).await.unwrap();
        assert!(store.load(&id).await.unwrap().is_some());
        assert_eq!(
            store
                .load_for_provider("mock")
                .await
                .unwrap()
                .unwrap()
                .cookies
                .get("sid")
                .map(String::as_str),
            Some("abc")
        );
        store.delete(&id).await.unwrap();
        assert!(store.load(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn file_crud() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(dir.path()).unwrap();
        let s = Session::new("mock").cookie("sid", "abc");
        let id = s.id.clone();
        store.save(&s).await.unwrap();
        let loaded = store.load(&id).await.unwrap().unwrap();
        assert_eq!(loaded.cookies.get("sid").map(String::as_str), Some("abc"));
        store.delete(&id).await.unwrap();
        assert!(store.load(&id).await.unwrap().is_none());
    }
}
