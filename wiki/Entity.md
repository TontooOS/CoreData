# Entity

`ManagedObject` is the CoreData-like row/document.

## ManagedObject

```rust
pub struct ManagedObject {
    pub object_id: String,   // UUID v4 from foundation::uuid
    pub entity: String,
    pub values: OrderedMap<String, FishValue>,
    pub rev: u64,
    pub updated_at: foundation::date::Date,
    pub deleted: bool,
}
```

### `new` / `with_id`

```rust
pub fn new(entity: impl Into<String>) -> Self
pub fn with_id(entity: impl Into<String>, id: impl Into<String>) -> Self
```

Creates a UUID v4. `rev` starts at `1`, `updated_at = Date::now()`.

### `updated_at_string` / `parse_timestamp`

```rust
pub fn updated_at_string(&self) -> String
pub fn parse_timestamp(text: &str) -> Option<foundation::date::Date>
```

`updated_at_string` renders RFC 3339 as `2026-01-01T12:00:00.000Z`.
`parse_timestamp` accepts that form **and** the nanosecond form written by
older builds (`2026-01-01T12:00:00.123456789+00:00`), so existing `.fico`
and SQLite stores keep loading. `Foundation::date::Date` has second
resolution; sub-second precision from old files is truncated on read.

### `get` / `set` / `remove`

```rust
pub fn get(&self, key: &str) -> Option<&FishValue>
pub fn set(&mut self, key: impl Into<String>, value: impl Into<FishValue>)
pub fn remove(&mut self, key: &str) -> Option<FishValue>
```

`set` bumps `rev` and `updated_at`. Typed helpers: `get_str`, `get_i64`, `get_bool`, `get_f64`.

### `to_fish_value` / `from_fish_value`

Serializes to `FishValue::Table` with meta keys `object_id`, `entity`, `rev`, `updated_at`, `deleted` plus user values. Used by `FicoStore`.

### `to_json` / `from_json`

```rust
pub fn to_json(&self) -> foundation::serialization::JsonValue
pub fn from_json(v: &foundation::serialization::JsonValue) -> Option<Self>
```

Manual conversion on top of Foundation's JSON, not derive macros. The
member shape is unchanged from the former serde output (`object_id`,
`entity`, `values`, `rev`, `updated_at`, `deleted`), so blobs written by
earlier builds still decrypt and load. `FishValue` floats that are not
finite encode as `null`. Used by `SqliteStore`.

### `mark_deleted`

Sets `deleted=true` and bumps `rev`. Soft delete kept for future sync.

## EntityDescription

```rust
pub struct EntityDescription { pub name: String, pub attributes: Vec<AttributeDescription> }
pub struct AttributeDescription { pub name: String, pub attribute_type: AttributeType, pub optional: bool }
pub enum AttributeType { String, Integer, Float, Boolean, Date, Binary, Transformable }
```

Lightweight schema hint, not enforced yet.

## Usage / Example

```rust
let mut obj = ManagedObject::new("Note");
obj.set("title", "Hello");
obj.set("count", 42);
assert_eq!(obj.get_str("title"), Some("Hello"));
```

## Cross References

- [Context.md](Context.md) – context creates and saves objects
- [FicoStore.md](FicoStore.md) – persistence via `to_fish_value`
- [SqliteStore.md](SqliteStore.md) – persistence via `to_json`
- [Fetch.md](Fetch.md) – predicates read `values`
