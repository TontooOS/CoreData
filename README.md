# Tontoo CoreData

Encrypted, per-app isolated persistence for TontooOS. FishFile `.fico` + SQLite backends, AES-256-GCM hardware-bound keys, `/Users/<user>/Library/Preferences/<bundleId>/storage.{fico,sqlite}`. Like Apple CoreData, simple.

## Made for TontooOS

Explore more at https://github.com/TontooOS/Libs

## Adding to Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
sdk = { path = "/Library/System/sdk", features = ["CoreData"] }
```

## License

TCL v26.1
