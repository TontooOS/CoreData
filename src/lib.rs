//! Tontoo CoreData – encrypted, per-app isolated persistence
//!
//! Stores are encrypted and isolated per bundle id at
//! `/Users/<user>/Library/Preferences/<bundleId>/storage.{fico,sqlite}`
//! Only supported on TontooOS / Arch Linux (WSL ArchLinux).
//!
//! # Quick Start
//! ```rust
//! use coredata::{PersistentContainer, StoreType};
//! let dir = tempfile::tempdir().unwrap();
//! std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
//! std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
//! let keyfile = dir.path().join("keyfile");
//! std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());
//! let bundle = format!("org.example.app.doc.{}", std::process::id());
//! let mut container = PersistentContainer::new_with_bundle(bundle.clone(), StoreType::Fico).unwrap();
//! let mut ctx = container.view_context();
//! let mut obj = ctx.create("Note");
//! obj.set("title", "Hello");
//! ctx.save_object(obj).unwrap();
//! ctx.save().unwrap();
//! let all = ctx.fetch_all("Note").unwrap();
//! assert_eq!(all.len(), 1);
//! ```

pub mod crypto;
pub mod entity;
pub mod error;
pub mod fetch;
pub mod fico_store;
pub mod lock;
pub mod perms;
pub mod paths;
pub mod sqlite_store;
pub mod store;
pub mod context;

pub mod ffi;

pub use context::{ManagedObjectContext, PersistentContainer};
pub use entity::{AttributeDescription, AttributeType, EntityDescription, ManagedObject};
pub use error::{CoreDataError, Result};
pub use fetch::{FetchRequest, Predicate, PredicateOperator, SortDescriptor};
pub use store::StoreType;

pub const COREDATA_VERSION: (u32, u32, u32) = (26, 1, 0);
pub const COREDATA_VERSION_STR: &str = "26.1.0";

#[cfg(target_os = "windows")]
compile_error!("CoreData only supports TontooOS / Arch Linux – use WSL ArchLinux (wsl -d archlinux)");

pub mod prelude {
    pub use crate::context::{ManagedObjectContext, PersistentContainer};
    pub use crate::entity::ManagedObject;
    pub use crate::error::{CoreDataError, Result};
    pub use crate::fetch::{FetchRequest, Predicate, PredicateOperator, SortDescriptor};
    pub use crate::store::StoreType;
    pub use crate::{COREDATA_VERSION, COREDATA_VERSION_STR};
}

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: once_cell::sync::Lazy<std::sync::Mutex<()>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(()));

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    fn test_env(bundle: &str) -> (TempDir, std::sync::MutexGuard<'static, ()>) {
        let guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
        std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
        let keyfile = dir.path().join("keyfile");
        std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());
        std::env::set_var("TONTOO_APP_BUNDLE_ID", bundle);
        (dir, guard)
    }

    #[test]
    fn fico_container_persist() {
        let (dir, _guard) = test_env("org.test.integration.fico");
        let mut c = PersistentContainer::new(None, StoreType::Fico).unwrap();
        {
            let mut ctx = c.view_context();
            let mut o = ctx.create("Book");
            o.set("title", "Rust");
            o.set("pages", 300);
            ctx.save_object(o).unwrap();
            ctx.save().unwrap();
        }
        // reload in new container same bundle
        let mut c2 = PersistentContainer::new(Some("org.test.integration.fico"), StoreType::Fico).unwrap();
        let ctx2 = c2.view_context();
        let books = ctx2.fetch_all("Book").unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].get_str("title"), Some("Rust"));
        drop(dir);
    }

    #[test]
    fn fico_rewrite_cycle_has_no_duplicates() {
        // Rewrite pattern (delete-all + reinsert, e.g. the Weather app):
        // soft-deleted rows must neither reappear in fetch_all nor
        // accumulate across save/load cycles.
        let (dir, _guard) = test_env("org.test.rewrite.nodupes");
        let mut c = PersistentContainer::new(None, StoreType::Fico).unwrap();
        {
            let mut ctx = c.view_context();
            for name in ["Tokyo", "Berlin", "New York"] {
                let mut o = ctx.create("SavedPlace");
                o.set("name", name);
                ctx.save_object(o).unwrap();
            }
            ctx.save().unwrap();
        }
        {
            let mut ctx = c.view_context();
            let ids: Vec<String> = ctx
                .fetch_all("SavedPlace")
                .unwrap()
                .iter()
                .map(|o| o.object_id.clone())
                .collect();
            assert_eq!(ids.len(), 3);
            for id in ids {
                ctx.delete(&id).unwrap();
            }
            for name in ["Tokyo", "Berlin", "New York", "Paris"] {
                let mut o = ctx.create("SavedPlace");
                o.set("name", name);
                ctx.save_object(o).unwrap();
            }
            ctx.save().unwrap();
            // Already filtered before reload.
            assert_eq!(ctx.fetch_all("SavedPlace").unwrap().len(), 4);
        }
        let mut c2 = PersistentContainer::new(Some("org.test.rewrite.nodupes"), StoreType::Fico).unwrap();
        let ctx2 = c2.view_context();
        let all = ctx2.fetch_all("SavedPlace").unwrap();
        assert_eq!(all.len(), 4);
        drop(dir);
    }

    #[test]
    fn sqlite_container_fetch_predicate() {        let (dir, _guard) = test_env("org.test.integration.sqlite");
        let mut c = PersistentContainer::new(None, StoreType::SQLite).unwrap();
        {
            let mut ctx = c.view_context();
            let mut a = ctx.create("User");
            a.set("name", "Alice");
            a.set("age", 30);
            let mut b = ctx.create("User");
            b.set("name", "Bob");
            b.set("age", 20);
            ctx.save_object(a).unwrap();
            ctx.save_object(b).unwrap();
            ctx.save().unwrap();
        }
        let mut c2 = PersistentContainer::new(Some("org.test.integration.sqlite"), StoreType::SQLite).unwrap();
        let ctx2 = c2.view_context();
        let req = FetchRequest::new("User").predicate(Predicate::new("age", PredicateOperator::GreaterThan, "25"));
        let res = ctx2.fetch(req).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].get_str("name"), Some("Alice"));
        drop(dir);
    }
}
