//! FicoStore – FishFile .fico backed encrypted store

use crate::crypto;
use crate::entity::ManagedObject;
use crate::error::{CoreDataError, Result};
use crate::fetch::FetchRequest;
use crate::paths;
use crate::perms;
use crate::store::{PersistentStore, StoreType};
use fishfile::{FishDocument, FishValue};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct FicoStore {
    bundle_id: String,
    path: PathBuf,
    // entity -> id -> object
    objects: HashMap<String, HashMap<String, ManagedObject>>,
    loaded: bool,
}

fn encode_id(id: &str) -> String {
    // Always prefix to guarantee valid identifier (UUIDs start with digit)
    format!("id_{}", id)
}

fn decode_id(key: &str) -> String {
    if let Some(stripped) = key.strip_prefix("id_") {
        stripped.to_string()
    } else {
        key.to_string()
    }
}

impl FicoStore {
    pub fn new(bundle_id: impl Into<String>) -> Self {
        let bundle_id = bundle_id.into();
        let path = paths::fico_path(&bundle_id);
        Self {
            bundle_id,
            path,
            objects: HashMap::new(),
            loaded: false,
        }
    }

    pub fn with_path(bundle_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            path: path.into(),
            objects: HashMap::new(),
            loaded: false,
        }
    }

    fn ensure_loaded(&mut self) -> Result<()> {
        if !self.loaded {
            self.load()?;
        }
        Ok(())
    }

    fn atomic_write_encrypted(&self, data: &[u8]) -> Result<()> {
        let dir = self.path.parent().ok_or_else(|| CoreDataError::InvalidPath("no parent".into()))?;
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(dir)?.permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(dir, perms);
        }
        let tmp = self.path.with_extension("fico.tmp");
        std::fs::write(&tmp, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&tmp) {
                let mut p = meta.permissions();
                p.set_mode(0o600);
                let _ = std::fs::set_permissions(&tmp, p);
            }
        }
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

impl PersistentStore for FicoStore {
    fn load(&mut self) -> Result<()> {
        perms::enforce_owner_or_fail(&self.bundle_id)?;
        let _lock = crate::lock::StorageLock::shared(&self.bundle_id)?;
        if !self.path.exists() {
            self.objects.clear();
            self.loaded = true;
            return Ok(());
        }
        let raw = std::fs::read(&self.path)?;
        let decrypted = crypto::decrypt(&raw, &self.bundle_id)?;
        let text = String::from_utf8(decrypted).map_err(|e| CoreDataError::crypto(format!("utf8: {e}")))?;
        let doc = FishDocument::parse(&text)?;
        let mut map: HashMap<String, HashMap<String, ManagedObject>> = HashMap::new();
        for (entity_name, entity_val) in doc.root().iter() {
            if let FishValue::Table(entity_table) = entity_val {
                for (obj_key, obj_val) in entity_table.iter() {
                    let obj_id = decode_id(obj_key);
                    if let Some(obj) = ManagedObject::from_fish_value(entity_name, &obj_id, obj_val) {
                        map.entry(entity_name.clone()).or_default().insert(obj_id.clone(), obj);
                    }
                }
            }
        }
        self.objects = map;
        self.loaded = true;
        Ok(())
    }

    fn save(&self) -> Result<()> {
        perms::enforce_owner_or_fail(&self.bundle_id)?;
        let _lock = crate::lock::StorageLock::exclusive(&self.bundle_id)?;
        paths::ensure_storage_dir(&self.bundle_id)?;
        let mut root: IndexMap<String, FishValue> = IndexMap::new();
        for (entity, objs) in &self.objects {
            let mut entity_table: IndexMap<String, FishValue> = IndexMap::new();
            for (id, obj) in objs {
                // Purge soft-deleted tombstones so rewrite-style callers
                // (delete-all + reinsert) cannot accumulate duplicates.
                if obj.deleted {
                    continue;
                }
                entity_table.insert(encode_id(id), obj.to_fish_value());
            }
            root.insert(entity.clone(), FishValue::Table(entity_table));
        }
        let doc = FishDocument::from_table(root);
        let text = doc.to_string();
        let encrypted = crypto::encrypt(text.as_bytes(), &self.bundle_id)?;
        self.atomic_write_encrypted(&encrypted)
    }

    fn insert(&mut self, obj: ManagedObject) -> Result<()> {
        self.ensure_loaded()?;
        perms::enforce_owner_or_fail(&self.bundle_id)?;
        let e = obj.entity.clone();
        let id = obj.object_id.clone();
        self.objects.entry(e).or_default().insert(id, obj);
        Ok(())
    }

    fn update(&mut self, obj: ManagedObject) -> Result<()> {
        self.ensure_loaded()?;
        perms::enforce_owner_or_fail(&self.bundle_id)?;
        let e = obj.entity.clone();
        let id = obj.object_id.clone();
        let entry = self.objects.entry(e).or_default();
        if !entry.contains_key(&id) {
            return Err(CoreDataError::NotFound(id));
        }
        entry.insert(id, obj);
        Ok(())
    }

    fn delete(&mut self, object_id: &str) -> Result<()> {
        self.ensure_loaded()?;
        perms::enforce_owner_or_fail(&self.bundle_id)?;
        for objs in self.objects.values_mut() {
            if let Some(o) = objs.get_mut(object_id) {
                o.mark_deleted();
                return Ok(());
            }
        }
        Err(CoreDataError::NotFound(object_id.to_string()))
    }

    fn fetch(&self, request: &FetchRequest) -> Result<Vec<ManagedObject>> {
        let objs = self.fetch_all(&request.entity)?;
        Ok(request.apply(objs))
    }

    fn fetch_all(&self, entity: &str) -> Result<Vec<ManagedObject>> {
        // For read, also enforce? read should also be denied for foreign
        perms::enforce_owner_or_fail(&self.bundle_id)?;
        let mut out = Vec::new();
        if let Some(map) = self.objects.get(entity) {
            for obj in map.values() {
                // Soft-deleted rows stay out of results (SQLite parity).
                if obj.deleted {
                    continue;
                }
                out.push(obj.clone());
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
        StoreType::Fico
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::ManagedObject;
    use tempfile::TempDir;

    fn setup() -> (TempDir, String, FicoStore, std::sync::MutexGuard<'static, ()>) {
        let guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
        std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
        let bundle = "org.test.fico";
        let keyfile = dir.path().join("keyfile");
        std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());
        let store = FicoStore::new(bundle);
        (dir, bundle.to_string(), store, guard)
    }

    #[test]
    fn fico_roundtrip() {
        let (_dir, _bundle, mut store, _guard) = setup();
        let mut obj = ManagedObject::new("Note");
        obj.set("title", "Hello");
        obj.set("count", 42);
        store.insert(obj.clone()).unwrap();
        store.save().unwrap();

        let mut store2 = FicoStore::new("org.test.fico");
        store2.load().unwrap();
        let fetched = store2.fetch_all("Note").unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].get_str("title"), Some("Hello"));
        assert_eq!(fetched[0].get_i64("count"), Some(42));
    }

    #[test]
    fn fico_fetch_predicate() {
        let (_dir, _bundle, mut store, _guard) = setup();
        let mut a = ManagedObject::new("Person");
        a.set("name", "Alice");
        a.set("age", 30);
        let mut b = ManagedObject::new("Person");
        b.set("name", "Bob");
        b.set("age", 20);
        store.insert(a).unwrap();
        store.insert(b).unwrap();
        store.save().unwrap();

        let mut store2 = FicoStore::new("org.test.fico");
        store2.load().unwrap();
        let req = crate::fetch::FetchRequest::new("Person").predicate(crate::fetch::Predicate::new("age", crate::fetch::PredicateOperator::GreaterThan, "25"));
        let res = store2.fetch(&req).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].get_str("name"), Some("Alice"));
    }

    #[test]
    fn fico_encrypted_on_disk() {
        let (_dir, _bundle, mut store, _guard) = setup();
        let mut obj = ManagedObject::new("Secret");
        obj.set("data", "top secret");
        store.insert(obj).unwrap();
        store.save().unwrap();
        let raw = std::fs::read(store.path.clone()).unwrap();
        // raw should not contain plaintext
        assert!(!raw.windows(10).any(|w| w == b"top secret"));
        assert!(raw.starts_with(b"CDF1"));
    }
}
