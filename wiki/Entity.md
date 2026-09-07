# Entity

`ManagedObject` is the CoreData-like row/document.

## ManagedObject

```rust
pub struct ManagedObject {
    pub object_id: String,   // UUID v4
    pub entity: String,
    pub values: IndexMap<String, FishValue>,
    pub rev: u64,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
}
```

### `new` / `with_id`

```rust
pub fn new(entity: impl Into<String>) -> Self
pub fn with_id(entity: impl Into<String>, id: impl Into<String>) -> Self
```

Creates UUID v4. `rev` starts at `1`, `updated_at = Utc::now()`.

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
pub fn to_json(&self) -> serde_json::Value
pub fn from_json(v: &serde_json::Value) -> Option<Self>
```

Used by `SqliteStore` blob.

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
