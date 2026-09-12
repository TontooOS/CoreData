# Tontoo CoreData – Wiki

Encrypted, per-app isolated persistence for TontooOS. FishFile `.fico` and SQLite backends with AES-256-GCM, hardware-bound keys (TPM 2.0 / machine-id) at `/Users/<user>/Library/Preferences/<bundleId>/storage.{fico,sqlite}`. CoreData-like API with `PersistentContainer` and `ManagedObjectContext`.

- Repository: https://github.com/TontooOS/CoreData
- License: TCL v26.1
- Version: 26.1.0

## Feature Index

| Feature | File | Description |
|---|---|---|
| Main index | [MAIN.md](MAIN.md) | This page |
| Rules | [RULE.md](RULE.md) | Wiki design system |
| Paths | [Paths.md](Paths.md) | Bundle resolution and storage paths |
| Crypto | [Crypto.md](Crypto.md) | Encryption, hardware-bound HKDF, file format |
| Entity | [Entity.md](Entity.md) | ManagedObject and attribute types |
| Fetch | [Fetch.md](Fetch.md) | FetchRequest, predicates and sorting |
| Context | [Context.md](Context.md) | PersistentContainer and ManagedObjectContext |
| FicoStore | [FicoStore.md](FicoStore.md) | FishFile .fico backend |
| SqliteStore | [SqliteStore.md](SqliteStore.md) | SQLite blob-encrypted backend |
| Perms | [Perms.md](Perms.md) | FishPerms Storage isolation |
| Ffi | [Ffi.md](Ffi.md) | C header and cdylib |

## Quick Start

```rust
use coredata::{PersistentContainer, StoreType};

fn main() -> coredata::Result<()> {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
    std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
    let keyfile = dir.path().join("keyfile");
    std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());
    std::env::set_var("TONTOO_APP_BUNDLE_ID", "org.example.app");

    let mut container = PersistentContainer::new(None, StoreType::Fico)?;
    let mut ctx = container.view_context();
    let mut obj = ctx.create("Note");
    obj.set("title", "Hello");
    obj.set("done", false);
    ctx.save_object(obj)?;
    ctx.save()?;

    let notes = ctx.fetch_all("Note")?;
    assert_eq!(notes.len(), 1);
    Ok(())
}
```

```c
#include "coredata.h"
ContainerHandle *h = coredata_open("org.example.app", "fico");
coredata_set(h, "Note", "id-123", "title", "\"Hello\"");
coredata_save(h);
char *v = coredata_get(h, "Note", "id-123", "title");
coredata_free_string(v);
coredata_close(h);
```

See [Context.md](Context.md), [FicoStore.md](FicoStore.md) and [Crypto.md](Crypto.md) for details.

## Changelog

- 2026-09-12: System-wide stores (`PersistentContainer::new_system_with_bundle`, `/System/Preferences/<bundle>/storage.{fico,sqlite}`, `coredata_open_system`): encrypted, 0o700 directory, no bundle-owner check
- 2026-09-10: `FicoStore::fetch_all` excludes soft-deleted rows and `save` purges tombstones (fixes duplicate rows for rewrite-style callers, e.g. the Weather app)
- 2026-08-26: Initial wiki for CoreData 26.1.0 – Fico/SQLite, Crypto, FishPerms isolation
