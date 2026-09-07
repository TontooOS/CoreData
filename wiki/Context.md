# Context

`PersistentContainer` and `ManagedObjectContext` – the main entry point.

## PersistentContainer

```rust
pub struct PersistentContainer { bundle_id: String, store_type: StoreType, store: Box<dyn PersistentStore> }
```

### `new` / `new_with_bundle` / `new_with_path`

```rust
pub fn new(bundle_id: Option<&str>, store_type: StoreType) -> Result<Self>
pub fn new_with_bundle(bundle_id: String, store_type: StoreType) -> Result<Self>
pub fn new_with_path(bundle_id: &str, store_type: StoreType, path: PathBuf) -> Result<Self>
```

`new(None, ..)` resolves bundle via `paths::resolve_bundle_id`. Checks FishPerms `Storage` silent policy, calls `store.load()`.

### `view_context`

```rust
pub fn view_context(&mut self) -> ManagedObjectContext<'_>
```

Single context (no concurrency yet) like `NSManagedObjectContext(viewContext)`.

### `save` / `load`

```rust
pub fn save(&self) -> Result<()>
pub fn load(&mut self) -> Result<()>
```

## ManagedObjectContext

```rust
pub struct ManagedObjectContext<'a> { store: &'a mut dyn PersistentStore }
```

### `create` / `insert` / `save_object` / `update` / `delete`

```rust
pub fn create(&mut self, entity: impl Into<String>) -> ManagedObject
pub fn insert(&mut self, obj: ManagedObject) -> Result<()>
pub fn save_object(&mut self, obj: ManagedObject) -> Result<()> // update or insert
pub fn update(&mut self, obj: ManagedObject) -> Result<()>
pub fn delete(&mut self, object_id: &str) -> Result<()>
```

`create` returns not yet inserted object; `save_object` handles both.

### `fetch` / `fetch_all` / `count` / `object` / `save`

```rust
pub fn fetch(&self, req: FetchRequest) -> Result<Vec<ManagedObject>>
pub fn fetch_all(&self, entity: &str) -> Result<Vec<ManagedObject>>
pub fn count(&self, req: FetchRequest) -> Result<usize>
pub fn object(&self, entity: &str, id: &str) -> Result<ManagedObject>
pub fn save(&self) -> Result<()>
```

## Usage / Example

```rust
let mut container = PersistentContainer::new_with_bundle("org.example.app".into(), StoreType::Fico)?;
let mut ctx = container.view_context();
let mut o = ctx.create("Note");
o.set("title", "Hi");
ctx.save_object(o)?;
ctx.save()?;
let all = ctx.fetch_all("Note")?;
```

## Cross References

- [Paths.md](Paths.md) – bundle resolution
- [Entity.md](Entity.md) – object lifecycle
- [Fetch.md](Fetch.md) – queries
- [FicoStore.md](FicoStore.md) – Fico backend
- [SqliteStore.md](SqliteStore.md) – SQLite backend
- [Perms.md](Perms.md) – isolation check on open
