# Perms

FishPerms Storage isolation – silent allow for owner, silent deny for foreign, never popup.

## Goal

Requirement: `C:\Users\arlo1\Documents\TontooServices\FishPerms` must not show popup for own `storage.{fico,sqlite}` access, but must block other apps' access.

## Design

New permission `Storage` (not yet in `protocol/src/lib.rs:14` which has 5 perms). CoreData checks before every `load/save/fetch/insert/delete`. If `caller == owner` → allow without daemon. Else → try daemon via Unix socket `/run/fishperms.sock` JSON framing (`write_msg/read_msg` style). Daemon should return `Allowed` only for owner, `Denied` otherwise. On daemon miss, fallback to `EnforceOwner` (deny foreign).

## API

```rust
pub fn check_storage_access(owner_bundle_id: &str) -> Result<()>
pub fn enforce_owner_or_fail(owner_bundle_id: &str) -> Result<()>
pub fn is_owner(owner_bundle_id: &str) -> bool
pub fn ensure_storage_policy(owner_bundle_id: &str)
```

`enforce_owner_or_fail` respects `TONTOO_COREDATA_ALLOW_FOREIGN=1` to bypass (used in tests). Otherwise resolves `caller` via `paths::resolve_bundle_id(None)` and compares.

`check_via_daemon` (unix only) connects, sends `{v:1, method:"check", params:{permission:"storage"}}`, handles `Result` states `allowed/denied`. On Windows, directly returns `Err`.

## FishPerms Daemon Change Needed

Add `Permission::Storage` to `protocol`, and in `daemon/policy.rs:110` handle `effective` with:

```rust
if permission == Storage { if app == owner { Allowed } else { Denied } }
```

No `Ask` for storage.

## Errors

- `PermissionDenied { caller, owner }` on foreign access.

## Usage / Example

```rust
// inside FicoStore::save
perms::enforce_owner_or_fail(&self.bundle_id)?;
```

## Cross References

- [Paths.md](Paths.md) – caller/owner bundle id
- [FicoStore.md](FicoStore.md) – checks on load/save
- [SqliteStore.md](SqliteStore.md) – checks on insert/fetch
- [Context.md](Context.md) – container checks on creation
