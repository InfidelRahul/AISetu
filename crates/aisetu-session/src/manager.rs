//! SessionManager: create, load, update, validate, invalidate, delete.

use std::sync::Arc;

use crate::{
    session::{Session, SessionId},
    store::SessionStore,
};

pub struct SessionManager {
    store: Arc<dyn SessionStore>,
}

impl SessionManager {
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        Self { store }
    }

    pub async fn create(&self, session: Session) -> aisetu_core::Result<Session> {
        session.validate()?;
        self.store.save(&session).await?;
        tracing::info!(provider = %session.provider, "session created");
        Ok(session)
    }

    pub async fn load(&self, id: &SessionId) -> aisetu_core::Result<Session> {
        let session = self
            .store
            .load(id)
            .await?
            .ok_or_else(|| aisetu_core::SetuError::not_found("session not found"))?;
        session.validate()?;
        Ok(session)
    }

    pub async fn load_for_provider(&self, provider: &str) -> aisetu_core::Result<Option<Session>> {
        match self.store.load_for_provider(provider).await? {
            Some(session) => {
                if session.validate().is_ok() {
                    Ok(Some(session))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    pub async fn update(&self, mut session: Session) -> aisetu_core::Result<Session> {
        session.touch();
        session.validate()?;
        self.store.save(&session).await?;
        Ok(session)
    }

    pub async fn validate(&self, id: &SessionId) -> aisetu_core::Result<Session> {
        self.load(id).await
    }

    pub async fn invalidate(&self, id: &SessionId) -> aisetu_core::Result<()> {
        if let Some(mut session) = self.store.load(id).await? {
            session.invalidate();
            self.store.save(&session).await?;
        }
        Ok(())
    }

    pub async fn delete(&self, id: &SessionId) -> aisetu_core::Result<()> {
        self.store.delete(id).await
    }

    /// Supply a validated session to the transport layer, if one exists.
    pub async fn for_transport(&self, provider: &str) -> aisetu_core::Result<Option<Session>> {
        self.load_for_provider(provider).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemorySessionStore;

    #[tokio::test]
    async fn lifecycle() {
        let mgr = SessionManager::new(Arc::new(MemorySessionStore::new()));
        let session = mgr
            .create(Session::new("mock").cookie("sid", "abc"))
            .await
            .unwrap();
        let id = session.id.clone();
        mgr.validate(&id).await.unwrap();
        mgr.invalidate(&id).await.unwrap();
        assert!(mgr.load(&id).await.is_err());
        mgr.delete(&id).await.unwrap();
    }
}
