//! PersistentStore trait

use crate::entity::ManagedObject;
use crate::error::Result;
use crate::fetch::FetchRequest;

pub trait PersistentStore: Send {
    fn load(&mut self) -> Result<()>;
    fn save(&self) -> Result<()>;
    fn insert(&mut self, obj: ManagedObject) -> Result<()>;
    fn update(&mut self, obj: ManagedObject) -> Result<()>;
    fn delete(&mut self, object_id: &str) -> Result<()>;
    fn fetch(&self, request: &FetchRequest) -> Result<Vec<ManagedObject>>;
    fn fetch_all(&self, entity: &str) -> Result<Vec<ManagedObject>>;
    fn count(&self, request: &FetchRequest) -> Result<usize>;
    fn bundle_id(&self) -> &str;
    fn store_type(&self) -> StoreType;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreType {
    Fico,
    SQLite,
}

impl StoreType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoreType::Fico => "fico",
            StoreType::SQLite => "sqlite",
        }
    }
}
