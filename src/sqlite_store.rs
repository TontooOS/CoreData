//! SQLiteStore – rusqlite + AES-GCM blob encryption

use crate::crypto;
use crate::entity::ManagedObject;
use crate::error::Result;
use crate::fetch::FetchRequest;
use crate::paths;
use crate::perms;
use crate::store::{PersistentStore, StoreType};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

pub struct SqliteStore {
    bundle_id: String,
    path: PathBuf,
    conn: Option<Connection>,
    // system-wide store under /System/Preferences: no bundle-owner check,
    // directories resolve to the system root instead
    system: bool,
}

impl SqliteStore {
    pub fn new(bundle_id: impl Into<String>) -> Self {
        let bundle_id = bundle_id.into();
        let path = paths::sqlite_path(&bundle_id);
        Self {
            bundle_id,
            path,
            conn: None,
            system: false,
        }
    }

    pub fn with_path(bundle_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            path: path.into(),
            conn: None,
            system: false,
        }
    }

    /// System-wide store: `path` must point inside `/System/Preferences`.
    /// Bundle-owner checks are skipped (the 0o700 directory owned by the
    /// privileged writer enforces isolation).
    pub fn with_system_path(bundle_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            path: path.into(),
            conn: None,
            system: true,
        }
    }

    fn check_access(&self) -> Result<()> {
        perms::enforce_access(&self.bundle_id, self.system)
    }

    fn ensure_dir(&self) -> Result<PathBuf> {
        if self.system {
            paths::ensure_system_storage_dir(&self.bundle_id)
        } else {
            paths::ensure_storage_dir(&self.bundle_id)
        }
    }

    fn ensure_conn(&mut self) -> Result<()> {
        if self.conn.is_some() {
            return Ok(());
        }
        self.open()?;
        Ok(())
    }

    fn open(&mut self) -> Result<()> {
        self.check_access()?;
        self.ensure_dir()?;
        let need_init = !self.path.exists();
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                entity TEXT NOT NULL,
                data BLOB NOT NULL,
                rev INTEGER NOT NULL,
                updated_at TEXT NOT NULL,
                deleted INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_entity ON objects(entity)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_deleted ON objects(deleted)", [])?;
        if need_init {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&self.path) {
                    let mut p = meta.permissions();
                    p.set_mode(0o600);
                    let _ = std::fs::set_permissions(&self.path, p);
                }
            }
        }
        self.conn = Some(conn);
        Ok(())
    }

    fn encrypt_blob(&self, json: &serde_json::Value) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(json).map_err(crate::error::CoreDataError::Serde)?;
        crypto::encrypt(&bytes, &self.bundle_id)
    }

    fn decrypt_blob(&self, blob: &[u8]) -> Result<ManagedObject> {
        let dec = crypto::decrypt(blob, &self.bundle_id)?;
        let v: serde_json::Value = serde_json::from_slice(&dec)?;
        ManagedObject::from_json(&v).ok_or_else(|| crate::error::CoreDataError::crypto("invalid object json"))
    }
}

impl PersistentStore for SqliteStore {
    fn load(&mut self) -> Result<()> {
        self.ensure_conn()?;
        Ok(())
    }

    fn save(&self) -> Result<()> {
        self.check_access()?;
        if let Some(conn) = &self.conn {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
        Ok(())
    }

    fn insert(&mut self, obj: ManagedObject) -> Result<()> {
        self.check_access()?;
        self.ensure_conn()?;
        let conn = self.conn.as_ref().ok_or_else(|| crate::error::CoreDataError::custom("not loaded"))?;
        let blob = self.encrypt_blob(&obj.to_json())?;
        conn.execute(
            "INSERT OR REPLACE INTO objects (id, entity, data, rev, updated_at, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![obj.object_id, obj.entity, blob, obj.rev as i64, obj.updated_at.to_rfc3339(), obj.deleted as i32],
        )?;
        Ok(())
    }

    fn update(&mut self, obj: ManagedObject) -> Result<()> {
        // same as insert
        self.insert(obj)
    }

    fn delete(&mut self, object_id: &str) -> Result<()> {
        self.check_access()?;
        self.ensure_conn()?;
        let conn = self.conn.as_ref().ok_or_else(|| crate::error::CoreDataError::custom("not loaded"))?;
        // fetch, mark deleted, re-encrypt
        let mut stmt = conn.prepare("SELECT data FROM objects WHERE id = ?1")?;
        let blob: Option<Vec<u8>> = stmt.query_row(params![object_id], |row| row.get(0)).optional()?;
        if let Some(b) = blob {
            let mut obj = self.decrypt_blob(&b)?;
            obj.mark_deleted();
            let new_blob = self.encrypt_blob(&obj.to_json())?;
            conn.execute(
                "UPDATE objects SET data = ?1, rev = ?2, updated_at = ?3, deleted = 1 WHERE id = ?4",
                params![new_blob, obj.rev as i64, obj.updated_at.to_rfc3339(), object_id],
            )?;
            Ok(())
        } else {
            Err(crate::error::CoreDataError::NotFound(object_id.to_string()))
        }
    }

    fn fetch(&self, request: &FetchRequest) -> Result<Vec<ManagedObject>> {
        self.check_access()?;
        let all = self.fetch_all(&request.entity)?;
        Ok(request.apply(all))
    }

    fn fetch_all(&self, entity: &str) -> Result<Vec<ManagedObject>> {
        self.check_access()?;
        let conn = self.conn.as_ref().ok_or_else(|| crate::error::CoreDataError::custom("not loaded"))?;
        let mut stmt = conn.prepare("SELECT data FROM objects WHERE entity = ?1 AND deleted = 0")?;
        let rows = stmt.query_map(params![entity], |row| row.get::<_, Vec<u8>>(0))?;
        let mut out = Vec::new();
        for r in rows {
            let blob = r?;
            if let Ok(obj) = self.decrypt_blob(&blob) {
                // also respect include_deleted already filtered
                out.push(obj);
            }
        }
        Ok(out)
    }

    fn count(&self, request: &FetchRequest) -> Result<usize> {
        Ok(self.fetch(request)?.len())
    }

    fn bundle_id(&self) -> &str {
        &self.bundle_id
    }

    fn store_type(&self) -> StoreType {
        StoreType::SQLite
    }
}

impl Drop for SqliteStore {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::ManagedObject;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore, std::sync::MutexGuard<'static, ()>) {
        let guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
        std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
        let keyfile = dir.path().join("keyfile");
        std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());
        let store = SqliteStore::new("org.test.sqlite");
        (dir, store, guard)
    }

    #[test]
    fn sqlite_roundtrip() {
        let (_dir, mut store, _guard) = setup();
        store.load().unwrap();
        let mut obj = ManagedObject::new("Note");
        obj.set("title", "Hello SQLite");
        obj.set("count", 99);
        store.insert(obj.clone()).unwrap();
        store.save().unwrap();
        let fetched = store.fetch_all("Note").unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].get_str("title"), Some("Hello SQLite"));
    }

    #[test]
    fn sqlite_delete_marks() {
        let (_dir, mut store, _guard) = setup();
        store.load().unwrap();
        let mut obj = ManagedObject::new("Item");
        obj.set("name", "to delete");
        let id = obj.object_id.clone();
        store.insert(obj).unwrap();
        store.delete(&id).unwrap();
        let fetched = store.fetch_all("Item").unwrap();
        assert_eq!(fetched.len(), 0);
    }

    #[test]
    fn sqlite_encrypted() {
        let (_dir, mut store, _guard) = setup();
        store.load().unwrap();
        let mut obj = ManagedObject::new("Secret");
        obj.set("data", "very secret sqlite");
        store.insert(obj).unwrap();
        drop(store);
        // read raw file, ensure not plaintext
        let path = paths::sqlite_path("org.test.sqlite");
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(6).any(|w| w == b"secret"));
    }
}
