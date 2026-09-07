# Fetch

FetchRequest with predicates and sort, inspired by `NSPredicate` / `NSSortDescriptor`.

## Predicate

```rust
pub struct Predicate { pub key: String, pub op: PredicateOperator, pub value: String, pub case_insensitive: bool }
pub enum PredicateOperator { Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual, Contains, BeginsWith, EndsWith, In }
```

### `new` / `case_insensitive` / `from_format`

```rust
pub fn new(key: impl Into<String>, op: PredicateOperator, value: impl Into<String>) -> Self
pub fn case_insensitive(mut self, v: bool) -> Self
pub fn from_format(format: &str) -> Result<Self> // "age > 30", "name == 'Alice'"
```

`evaluate(&self, &ManagedObject) -> bool` does string and numeric compare (tries `f64` parse). `In` splits by `,`.

## SortDescriptor

```rust
pub struct SortDescriptor { pub key: String, pub ascending: bool, pub case_insensitive: bool }
pub fn new(key: impl Into<String>, ascending: bool) -> Self
```

## FetchRequest

```rust
pub struct FetchRequest {
    pub entity: String,
    pub predicate: Option<Predicate>,
    pub predicates: Vec<Predicate>, // AND
    pub sort_descriptors: Vec<SortDescriptor>,
    pub fetch_limit: Option<usize>,
    pub fetch_offset: usize,
    pub include_deleted: bool,
}
```

### Builders

```rust
pub fn new(entity: impl Into<String>) -> Self
pub fn predicate(mut self, p: Predicate) -> Self
pub fn and_predicate(mut self, p: Predicate) -> Self
pub fn sorted_by(mut self, d: SortDescriptor) -> Self
pub fn limit(mut self, n: usize) -> Self
pub fn offset(mut self, n: usize) -> Self
pub fn include_deleted(mut self, v: bool) -> Self
```

### `matches` / `apply`

```rust
pub fn matches(&self, obj: &ManagedObject) -> bool
pub fn apply(&self, objects: Vec<ManagedObject>) -> Vec<ManagedObject>
```

`apply` filters, sorts (numeric if possible), then offset/limit.

## Usage / Example

```rust
let req = FetchRequest::new("Person")
    .predicate(Predicate::new("age", PredicateOperator::GreaterThan, "25"))
    .sorted_by(SortDescriptor::new("name", true))
    .limit(10);
let res = ctx.fetch(req).unwrap();
```

## Cross References

- [Entity.md](Entity.md) – evaluated against `ManagedObject::get`
- [Context.md](Context.md) – `ctx.fetch(req)`
- [FicoStore.md](FicoStore.md) / [SqliteStore.md](SqliteStore.md) – store implements `fetch`
