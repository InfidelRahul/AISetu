//! Production-grade local secret handling, separate from ordinary configuration.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A stored secret blob. The value is zeroized on drop.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct StoredSecret {
    #[zeroize(skip)]
    pub kind: SecretKind,
    #[zeroize(skip)]
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    ApiCredential,
    ProviderSession,
    Configuration,
    RuntimeState,
}

impl std::fmt::Debug for StoredSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredSecret")
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

/// File-backed secret store with restrictive permissions.
///
/// Secrets are not treated as ordinary configuration. API credentials,
/// provider sessions, configuration, and runtime state live under distinct
/// subdirectories.
pub struct SecretStore {
    root: PathBuf,
}

impl SecretStore {
    pub fn open(root: impl Into<PathBuf>) -> aisetu_core::Result<Self> {
        let root = root.into();
        for kind in [
            SecretKind::ApiCredential,
            SecretKind::ProviderSession,
            SecretKind::Configuration,
            SecretKind::RuntimeState,
        ] {
            let dir = root.join(kind_dir(kind));
            std::fs::create_dir_all(&dir).map_err(|e| {
                aisetu_core::SetuError::configuration(format!(
                    "failed to create secret dir {}: {e}",
                    dir.display()
                ))
            })?;
            restrict_dir(&dir);
        }
        Ok(Self { root })
    }

    pub fn put(&self, secret: &StoredSecret) -> aisetu_core::Result<()> {
        let path = self.path_for(secret.kind, &secret.name);
        let json = serde_json::to_string(secret)
            .map_err(|e| aisetu_core::SetuError::internal(format!("serialize secret: {e}")))?;
        atomic_write(&path, json.as_bytes())?;
        Ok(())
    }

    pub fn get(&self, kind: SecretKind, name: &str) -> aisetu_core::Result<Option<StoredSecret>> {
        let path = self.path_for(kind, name);
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| aisetu_core::SetuError::internal(format!("read secret: {e}")))?;
        let secret = serde_json::from_str(&raw)
            .map_err(|e| aisetu_core::SetuError::parse_failure(format!("corrupt secret: {e}")))?;
        Ok(Some(secret))
    }

    pub fn delete(&self, kind: SecretKind, name: &str) -> aisetu_core::Result<()> {
        let path = self.path_for(kind, name);
        if path.exists() {
            overwrite_and_remove(&path)?;
        }
        Ok(())
    }

    fn path_for(&self, kind: SecretKind, name: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        let digest = hex::encode(hasher.finalize());
        self.root
            .join(kind_dir(kind))
            .join(format!("{digest}.secret"))
    }

    pub fn split_from_config(
        api_key: Option<&str>,
        extra: BTreeMap<String, String>,
    ) -> Vec<StoredSecret> {
        let mut out = Vec::new();
        if let Some(key) = api_key {
            out.push(StoredSecret {
                kind: SecretKind::ApiCredential,
                name: "api_key".into(),
                value: key.to_string(),
            });
        }
        for (k, v) in extra {
            out.push(StoredSecret {
                kind: SecretKind::Configuration,
                name: k,
                value: v,
            });
        }
        out
    }
}

fn kind_dir(kind: SecretKind) -> &'static str {
    match kind {
        SecretKind::ApiCredential => "api-credentials",
        SecretKind::ProviderSession => "provider-sessions",
        SecretKind::Configuration => "configuration",
        SecretKind::RuntimeState => "runtime-state",
    }
}

fn restrict_dir(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> aisetu_core::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)
        .map_err(|e| aisetu_core::SetuError::internal(format!("write secret: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path)
        .map_err(|e| aisetu_core::SetuError::internal(format!("commit secret: {e}")))?;
    Ok(())
}

fn overwrite_and_remove(path: &Path) -> aisetu_core::Result<()> {
    if let Ok(meta) = std::fs::metadata(path) {
        let zeros = vec![0u8; meta.len() as usize];
        let _ = std::fs::write(path, zeros);
    }
    std::fs::remove_file(path)
        .map_err(|e| aisetu_core::SetuError::internal(format!("remove secret: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::open(dir.path()).unwrap();
        let secret = StoredSecret {
            kind: SecretKind::ApiCredential,
            name: "api_key".into(),
            value: "sk-test".into(),
        };
        store.put(&secret).unwrap();
        let loaded = store
            .get(SecretKind::ApiCredential, "api_key")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.value, "sk-test");
        let debug = format!("{loaded:?}");
        assert!(!debug.contains("sk-test"));
        store.delete(SecretKind::ApiCredential, "api_key").unwrap();
        assert!(store
            .get(SecretKind::ApiCredential, "api_key")
            .unwrap()
            .is_none());
    }

    #[test]
    fn kinds_are_separated() {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::open(dir.path()).unwrap();
        store
            .put(&StoredSecret {
                kind: SecretKind::ApiCredential,
                name: "x".into(),
                value: "a".into(),
            })
            .unwrap();
        store
            .put(&StoredSecret {
                kind: SecretKind::ProviderSession,
                name: "x".into(),
                value: "b".into(),
            })
            .unwrap();
        assert_eq!(
            store
                .get(SecretKind::ApiCredential, "x")
                .unwrap()
                .unwrap()
                .value,
            "a"
        );
        assert_eq!(
            store
                .get(SecretKind::ProviderSession, "x")
                .unwrap()
                .unwrap()
                .value,
            "b"
        );
    }
}
