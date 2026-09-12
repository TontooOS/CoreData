# Ffi

C header and cdylib for `/Library/System/coredata.library`.

## Header

`Headers/coredata.h` exposes opaque `ContainerHandle`.

## Functions

| Function | Return | Meaning |
|---|---|---|
| `coredata_version()` | `const char*` | Static version `"26.1.0"` |
| `coredata_last_error()` | `char*` | Allocated error string or NULL, free with `coredata_free_string` |
| `coredata_free_string(char*)` | `void` | Free string from `coredata_get` / `coredata_last_error` |
| `coredata_open(bundle_id, store_type)` | `ContainerHandle*` | Open container, `store_type="fico"` or `"sqlite"`, NULL on error |
| `coredata_open_system(bundle_id, store_type)` | `ContainerHandle*` | Open system-wide container at `/System/Preferences/<bundle_id>`, NULL on error |
| `coredata_close(handle)` | `void` | Close and free |
| `coredata_save(handle)` | `int` | `0` ok, `-1` error |
| `coredata_set(handle, entity, object_id, key, json_value)` | `int` | `0` ok, `-1` error; `json_value` is JSON or plain string |
| `coredata_get(handle, entity, object_id, key)` | `char*` | JSON string or NULL, free with `coredata_free_string` |
| `coredata_delete(handle, entity, object_id)` | `int` | `0` ok, `-1` error |
| `coredata_fetch_count(handle, entity)` | `int` | Count or `-1` error |

## Memory Rules

| Allocation | Free with |
|---|---|
| `coredata_get` return | `coredata_free_string` |
| `coredata_last_error` return | `coredata_free_string` |
| `coredata_open` handle | `coredata_close` |
| `coredata_open_system` handle | `coredata_close` |

## Rust Side

`src/ffi.rs` holds `PersistentContainer` in `Box<ContainerHandle>` and uses `Mutex<Option<String>>` for `LAST_ERROR`. `coredata_set` parses `json_value` via `serde_json::from_str`, fallback to string, converts to `FishValue::from_json`.

## Usage / Example

```c
#include "coredata.h"
ContainerHandle *h = coredata_open("org.example.app", "fico");
if (!h) { char *e = coredata_last_error(); printf("%s\n", e); coredata_free_string(e); }
coredata_set(h, "Note", "id-123", "title", "\"Hello\"");
coredata_save(h);
char *v = coredata_get(h, "Note", "id-123", "title"); // "\"Hello\""
if (v) coredata_free_string(v);
coredata_close(h);
```

## Cross References

- [Context.md](Context.md) – Rust container
- [Entity.md](Entity.md) – object model
