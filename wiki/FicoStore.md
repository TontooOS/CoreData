# FicoStore

FishFile `.fico` encrypted backend.

## Overview

Uses `fishfile::FishDocument` per `C:\Users\arlo1\Documents\TontooLibs\FishFile\src\document.rs:34`. In-memory map `entity -> id -> ManagedObject`, persisted as:

```
entity {
  id_<uuid> {
    title: "Hello"
    rev: 1
    updated_at: "2026-08-26T..."
  }
}
```

Keys are `id_<uuid>` to stay valid fico identifiers (UUIDs start with digit). Encrypted with `crypto::encrypt` before atomic write.

## API

```rust
pub struct FicoStore { bundle_id: String, path: PathBuf, objects: HashMap<...> }
pub fn new(bundle_id: impl Into<String>) -> Self
pub fn with_path(bundle_id: impl Into<String>, path: impl Into<PathBuf>) -> Self
```

Implements `PersistentStore`:

| Method | Behavior |
|---|---|
| `load` | Checks `perms::enforce_owner_or_fail`, decrypts `CDF1` blob, parses `FishDocument`, decodes `id_` prefix |
| `save` | `ensure_storage_dir`, builds `FishDocument`, `to_string`, encrypt, `atomic_write_encrypted` (`0700` dir, `0600` file, `.tmp` + rename) |
| `insert` / `update` / `delete` | In-memory, `delete` marks `deleted=true` |
| `fetch` / `fetch_all` | `Request::apply` filtering |

## Atomic Write

Writes to `storage.fico.tmp` then `rename`. Same pattern as `FishPerms policy.rs:186`.

## Errors

- `PermissionDenied` if caller bundle != owner (unless `TONTOO_COREDATA_ALLOW_FOREIGN=1`)
- `Crypto` if wrong key
- `FishFile::Parse` if file corrupted

## Usage / Example

```rust
let mut store = FicoStore::new("org.example.app");
let mut obj = ManagedObject::new("Note");
obj.set("title", "Hi");
store.insert(obj).unwrap();
store.save().unwrap();
```

## Cross References

- [Crypto.md](Crypto.md) – encryption
- [Paths.md](Paths.md) – `fico_path`
- [Entity.md](Entity.md) – `to_fish_value` / `from_fish_value`
- [Perms.md](Perms.md) – isolation
