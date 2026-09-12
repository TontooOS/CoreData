//! ManagedObjectContext & PersistentContainer – CoreData-like API

use crate::entity::ManagedObject;
use crate::error::{CoreDataError, Result};
use crate::fetch::FetchRequest;
use crate::fico_store::FicoStore;
use crate::sqlite_store::SqliteStore;
use crate::store::{PersistentStore, StoreType};

/// High-level container that owns the store and provides contexts.
pub struct PersistentContainer {
    bundle_id: String,
    store_type: StoreType,
    // inner store boxed to allow switching
    store: Box<dyn PersistentStore>,
    // store path override for tests
    _path_override: Option<String>,
}

impl PersistentContainer {
    /// Create a new container for a bundle.
    /// If bundle_id is None, it will be resolved via TONTOO_APP_BUNDLE_ID / Info.tontoo.
    pub fn new(bundle_id: Option<&str>, store_type: StoreType) -> Result<Self> {
        let bundle_id = crate::paths::resolve_bundle_id(bundle_id)?;
        Self::new_with_bundle(bundle_id, store_type)
    }

    pub fn new_with_bundle(bundle_id: String, store_type: StoreType) -> Result<Self> {
        // perms: ensure owner can create – fishperms silent check
        crate::perms::ensure_storage_policy(&bundle_id);
        crate::perms::enforce_owner_or_fail(&bundle_id)?;

        let store: Box<dyn PersistentStore> = match store_type {
            StoreType::Fico => Box::new(FicoStore::new(bundle_id.clone())),
            StoreType::SQLite => Box::new(SqliteStore::new(bundle_id.clone())),
        };
        let mut c = Self {
            bundle_id,
            store_type,
            store,
            _path_override: None,
        };
        c.store.load()?;
        Ok(c)
    }

    /// For tests – inject custom path prefix via env
    pub fn new_with_path(bundle_id: &str, store_type: StoreType, path: std::path::PathBuf) -> Result<Self> {        crate::perms::enforce_owner_or_fail(bundle_id)?;
        let store: Box<dyn PersistentStore> = match store_type {
            StoreType::Fico => Box::new(FicoStore::with_path(bundle_id, path)),
            StoreType::SQLite => Box::new(SqliteStore::with_path(bundle_id, path)),
        };
        let mut c = Self {
            bundle_id: bundle_id.to_string(),
            store_type,
            store,
            _path_override: None,
        };
        c.store.load()?;
        Ok(c)
    }

    /// System-wide container shared by all users, stored encrypted at
    /// `/System/Preferences/<bundle_id>/storage.{fico,sqlite}`.
    /// Bundle-owner checks are skipped: only a privileged writer (e.g. a
    /// system daemon) can create the 0o700 directory, and unprivileged
    /// callers fail with an I/O permission error instead.
    pub fn new_system_with_bundle(bundle_id: String, store_type: StoreType) -> Result<Self> {
        crate::perms::ensure_storage_policy(&bundle_id);
        crate::paths::ensure_system_storage_dir(&bundle_id)?;
        let store: Box<dyn PersistentStore> = match store_type {
            StoreType::Fico => Box::new(FicoStore::with_system_path(
                bundle_id.clone(),
                crate::paths::system_fico_path(&bundle_id),
            )),
            StoreType::SQLite => Box::new(SqliteStore::with_system_path(
                bundle_id.clone(),
                crate::paths::system_sqlite_path(&bundle_id),
            )),
        };
        let mut c = Self {
            bundle_id,
            store_type,
            store,
            _path_override: None,
        };
        c.store.load()?;
        Ok(c)
    }

    pub fn bundle_id(&self) -> &str {
        &self.bundle_id
    }

    pub fn store_type(&self) -> StoreType {
        self.store_type
    }

    /// Get a context view – for now single context (no concurrency). Mirrors NSManagedObjectContext(viewContext)
    pub fn view_context(&mut self) -> ManagedObjectContext<'_> {
        ManagedObjectContext { store: &mut *self.store }
    }

    /// Save the underlying store
    pub fn save(&self) -> Result<()> {
        self.store.save()
    }

    pub fn load(&mut self) -> Result<()> {
        self.store.load()
    }
}

/// Context for object lifecycle – insert/update/delete/fetch.
pub struct ManagedObjectContext<'a> {
    store: &'a mut dyn PersistentStore,
}

impl<'a> ManagedObjectContext<'a> {
    pub fn insert(&mut self, obj: ManagedObject) -> Result<()> {
        self.store.insert(obj)
    }

    /// Insert new empty object for entity.
    pub fn insert_new(&mut self, entity: impl Into<String>) -> Result<ManagedObject> {
        let obj = ManagedObject::new(entity);
        // we return object without inserting? CoreData pattern: create and insert.
        // We'll insert and return clone.
        self.store.insert(obj.clone())?;
        Ok(obj)
    }

    /// Helper that creates, inserts and returns mutable handle via closure?
    pub fn create(&mut self, entity: impl Into<String>) -> ManagedObject {
        ManagedObject::new(entity)
    }

    pub fn save_object(&mut self, obj: ManagedObject) -> Result<()> {
        // try update, fallback to insert
        match self.store.update(obj.clone()) {
            Ok(()) => Ok(()),
            Err(CoreDataError::NotFound(_)) => self.store.insert(obj),
            Err(e) => Err(e),
        }
    }

    pub fn update(&mut self, obj: ManagedObject) -> Result<()> {
        self.store.update(obj)
    }

    pub fn delete(&mut self, object_id: &str) -> Result<()> {
        self.store.delete(object_id)
    }

    pub fn fetch(&self, req: FetchRequest) -> Result<Vec<ManagedObject>> {
        self.store.fetch(&req)
    }

    pub fn fetch_all(&self, entity: &str) -> Result<Vec<ManagedObject>> {
        self.store.fetch_all(entity)
    }

    pub fn count(&self, req: FetchRequest) -> Result<usize> {
        self.store.count(&req)
    }

    pub fn save(&self) -> Result<()> {
        self.store.save()
    }

    /// Fetch one by id
    pub fn object(&self, entity: &str, id: &str) -> Result<ManagedObject> {
        let all = self.store.fetch_all(entity)?;
        all.into_iter()
            .find(|o| o.object_id == id)
            .ok_or_else(|| CoreDataError::NotFound(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
        let guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
        std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
        let keyfile = dir.path().join("keyfile");
        std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());
        std::env::set_var("TONTOO_APP_BUNDLE_ID", "org.test.container");
        (dir, guard)
    }

    #[test]
    fn container_fico_flow() {
        let (_dir, _guard) = setup_env();
        let mut container = PersistentContainer::new(None, StoreType::Fico).unwrap();
        let mut ctx = container.view_context();
        let mut obj = ctx.create("Note");
        obj.set("title", "from ctx");
        ctx.save_object(obj.clone()).unwrap();
        ctx.save().unwrap();

        let fetched = ctx.fetch_all("Note").unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].get_str("title"), Some("from ctx"));
    }

    #[test]
    fn container_sqlite_flow() {
        let (_dir, _guard) = setup_env();
        std::env::set_var("TONTOO_APP_BUNDLE_ID", "org.test.container2");
        let mut container = PersistentContainer::new(None, StoreType::SQLite).unwrap();
        let mut ctx = container.view_context();
        let mut obj = ctx.create("Task");
        obj.set("done", false);
        obj.set("name", "test");
        ctx.save_object(obj).unwrap();
        ctx.save().unwrap();
        let all = ctx.fetch_all("Task").unwrap();
        assert_eq!(all.len(), 1);
    }
}
