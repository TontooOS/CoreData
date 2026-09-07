# Tontoo CoreData

Encrypted, per-app isolated persistence for TontooOS. FishFile `.fico` + SQLite backends, AES-256-GCM hardware-bound keys, `/Users/<user>/Library/Preferences/<bundleId>/storage.{fico,sqlite}`. Like Apple CoreData, simple.

## Made for TontooOS

Explore more at https://github.com/TontooOS/Libs

Wiki: [wiki/MAIN.md](wiki/MAIN.md)

## Adding to Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
sdk = { path = "/Library/System/sdk", features = ["CoreData"] }
```

Then at the crate root:

```rust
sdk::preinclude!();
use CoreData::{PersistentContainer, StoreType};
```

## Quick Start

```rust
use CoreData::{PersistentContainer, StoreType};

fn main() -> CoreData::Result<()> {
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

C:

```c
#include "coredata.h"
ContainerHandle *h = coredata_open("org.example.app", "fico");
coredata_set(h, "Note", "uuid-123", "title", "\"Hello\"");
coredata_save(h);
char *v = coredata_get(h, "Note", "uuid-123", "title");
coredata_free_string(v);
coredata_close(h);
```

See [wiki/MAIN.md](wiki/MAIN.md) for full API.

## Features

- Per-app isolation via bundle id (`Info.tontoo` / `TONTOO_APP_BUNDLE_ID`)
- Encrypted `CDF1` format, AES-256-GCM, HKDF per bundle, TPM 2.0 / machine-id bound
- `.fico` (FishFile) and SQLite backends
- `FetchRequest` with predicates and sort descriptors
- FishPerms `Storage` silent isolation (no popup, deny foreign)
- C header `Headers/coredata.h` (`cdylib`)

## License

TCL v26.1
