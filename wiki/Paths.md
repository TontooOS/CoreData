# Paths

Resolves bundle identifiers and storage locations in TontooOS style `/Users/<user>/Library/Preferences/<bundleId>/`.

## Overview

`paths.rs` mirrors `FishRunner` discovery: env `TONTOO_APP_BUNDLE_ID` set by `launch.rs:25` takes precedence over `Info.tontoo` discovery.

## API

### `resolve_bundle_id`

```rust
pub fn resolve_bundle_id(explicit: Option<&str>) -> Result<String>
```

Priority: `explicit` arg > `TONTOO_APP_BUNDLE_ID` env > `Info.tontoo` near exe (`TONTOO_APP_PATH` or walk up from `current_exe`). Returns `Err(NoBundle)` if none found.

**Example:**
```rust
let id = coredata::paths::resolve_bundle_id(None).unwrap();
let id2 = coredata::paths::resolve_bundle_id(Some("org.tontooos.dock")).unwrap();
```

### `preferences_root` / `tontoo_home`

```rust
pub fn preferences_root() -> PathBuf
pub fn tontoo_home() -> PathBuf
```

`preferences_root` respects `TONTOO_PREFERENCES_ROOT` override (used in tests), else `tontoo_home()/Library/Preferences`. `tontoo_home` checks `HOME` env first (accepts `/Users/...` even on Windows), else `dirs::home_dir()`.

### `storage_dir` / `storage_path` / `fico_path` / `sqlite_path`

```rust
pub fn storage_dir(bundle_id: &str) -> PathBuf
pub fn fico_path(bundle_id: &str) -> PathBuf  // storage.fico
pub fn sqlite_path(bundle_id: &str) -> PathBuf // storage.sqlite
pub fn meta_path(bundle_id: &str) -> PathBuf  // storage.meta
```

### `ensure_storage_dir`

```rust
pub fn ensure_storage_dir(bundle_id: &str) -> Result<PathBuf>
```

Creates dir with `0700` on Unix.

## File Layout

```
/Users/<user>/Library/Preferences/<bundleId>/storage.fico   # FishFile
/Users/<user>/Library/Preferences/<bundleId>/storage.sqlite # SQLite WAL
/Users/<user>/Library/Preferences/<bundleId>/storage.meta   # reserved for sync token
Permissions: 0700 dir, 0600 file
```

## Cross References

- [Crypto.md](Crypto.md) – encryption uses `bundle_id` as HKDF info
- [Perms.md](Perms.md) – checks caller vs owner bundle
- [Context.md](Context.md) – container resolves bundle via this module
