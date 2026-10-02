# SqliteStore

SQLite blob-encrypted backend.

## Overview

Tontoo `SQLKit` (dependency-light, zero third-party SQLite; ruslite-compatible
API, native B-Tree files). Table:

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

Each row's `data` is `crypto::encrypt(ManagedObject::to_json().stringify(false))`, rendered by `foundation::serialization::JsonValue`. WAL mode `journal_mode=WAL`.

## Backend Migration

Files written by the old `rusqlite` backend stay readable: SQLKit opens
them through its foreign B-Tree read path and migrates them to the native
layout on the first write. Files written by SQLKit stay real SQLite files
readable by `rusqlite`, the SQLite CLI, and CPython `sqlite3`. Covered by
`sqlite_rusqlite_file_stays_readable` and `sqlite_file_readable_by_rusqlite`
(`rusqlite` remains a dev-dependency for these migration tests only).

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
