# SqliteStore

SQLite blob-encrypted backend.

## Overview

`rusqlite` bundled (`libsqlite3-sys` with `bundled` feature). Table:

```sql
CREATE TABLE objects (
  id TEXT PRIMARY KEY,
  entity TEXT NOT NULL,
  data BLOB NOT NULL, -- AES-GCM encrypted JSON of ManagedObject
  rev INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_entity ON objects(entity);
```

Each row's `data` is `crypto::encrypt(serde_json::to_vec(ManagedObject))`. WAL mode `journal_mode=WAL`.

## API

```rust
pub struct SqliteStore { bundle_id: String, path: PathBuf, conn: Option<Connection> }
pub fn new(bundle_id: impl Into<String>) -> Self
pub fn with_path(bundle_id: impl Into<String>, path: impl Into<PathBuf>) -> Self
```

Implements `PersistentStore`:

| Method | Behavior |
|---|---|
| `open` | `ensure_storage_dir`, `Connection::open`, `PRAGMA journal_mode=WAL`, creates table |
| `insert` | `INSERT OR REPLACE` encrypted blob |
| `delete` | Fetch, `mark_deleted`, re-encrypt, `UPDATE deleted=1` |
| `fetch_all` | `SELECT data WHERE entity=? AND deleted=0`, decrypt each, `Request::apply` later |

## Permissions

`0600` on new file (Unix). Checks `enforce_owner_or_fail` on every op.

## Errors

- `Sqlite` on `CannotOpen`
- `Crypto` on decrypt failure
- `PermissionDenied` on foreign access

## Usage / Example

```rust
let mut store = SqliteStore::new("org.example.app");
store.load().unwrap();
let mut obj = ManagedObject::new("Task");
obj.set("done", false);
store.insert(obj).unwrap();
let tasks = store.fetch_all("Task").unwrap();
```

## Cross References

- [Crypto.md](Crypto.md) – per-row encryption
- [Paths.md](Paths.md) – `sqlite_path`
- [Context.md](Context.md) – container selects this store via `StoreType::SQLite`
