//! ManagedObject – CoreData-like entity instance

use chrono::{DateTime, Utc};
use fishfile::FishValue;
use indexmap::IndexMap;
use uuid::Uuid;

/// A single managed object (row / document).
#[derive(Debug, Clone)]
pub struct ManagedObject {
    /// Stable object ID – UUID v4
    pub object_id: String,
    /// Entity / table name
    pub entity: String,
    /// Attributes
    pub values: IndexMap<String, FishValue>,
    /// Revision for sync (Lamport)
    pub rev: u64,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Soft delete marker
    pub deleted: bool,
}

impl ManagedObject {
    pub fn new(entity: impl Into<String>) -> Self {
        Self {
            object_id: Uuid::new_v4().to_string(),
            entity: entity.into(),
            values: IndexMap::new(),
            rev: 1,
            updated_at: Utc::now(),
            deleted: false,
        }
    }

    pub fn with_id(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            object_id: id.into(),
            entity: entity.into(),
            values: IndexMap::new(),
            rev: 1,
            updated_at: Utc::now(),
            deleted: false,
        }
    }

    pub fn get(&self, key: &str) -> Option<&FishValue> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<FishValue>) {
        self.values.insert(key.into(), value.into());
        self.bump();
    }

    pub fn remove(&mut self, key: &str) -> Option<FishValue> {
        let v = self.values.shift_remove(key);
        if v.is_some() {
            self.bump();
        }
        v
    }

    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    fn bump(&mut self) {
        self.rev = self.rev.wrapping_add(1);
        self.updated_at = Utc::now();
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = true;
        self.bump();
    }

    /// Convert to FishValue::Table for Fico storage.
    pub fn to_fish_value(&self) -> FishValue {
        let mut t = IndexMap::new();
        t.insert("object_id".to_string(), FishValue::String(self.object_id.clone()));
        t.insert("entity".to_string(), FishValue::String(self.entity.clone()));
        t.insert("rev".to_string(), FishValue::Integer(self.rev as i64));
        t.insert("updated_at".to_string(), FishValue::String(self.updated_at.to_rfc3339()));
        t.insert("deleted".to_string(), FishValue::Bool(self.deleted));
        for (k, v) in &self.values {
            t.insert(k.clone(), v.clone());
        }
        FishValue::Table(t)
    }

    pub fn from_fish_value(entity_hint: &str, id: &str, v: &FishValue) -> Option<Self> {
        let table = v.as_table()?;
        let mut values = IndexMap::new();
        let mut rev = 1u64;
        let mut updated_at = Utc::now();
        let mut deleted = false;
        let mut object_id = id.to_string();
        for (k, val) in table {
            match k.as_str() {
                "object_id" => {
                    if let Some(s) = val.as_str() {
                        object_id = s.to_string();
                    }
                }
                "rev" => {
                    if let Some(i) = val.as_i64() {
                        rev = i as u64;
                    }
                }
                "updated_at" => {
                    if let Some(s) = val.as_str() {
                        if let Ok(dt) = s.parse::<DateTime<Utc>>() {
                            updated_at = dt;
                        }
                    }
                }
                "deleted" => {
                    if let Some(b) = val.as_bool() {
                        deleted = b;
                    }
                }
                "entity" => {}
                _ => {
                    values.insert(k.clone(), val.clone());
                }
            }
        }
        Some(Self {
            object_id,
            entity: entity_hint.to_string(),
            values,
            rev,
            updated_at,
            deleted,
        })
    }

    /// To JSON for SQLite blob (same shape as the former serde output).
    pub fn to_json(&self) -> serde_json::Value {
        let mut values = serde_json::Map::new();
        for (k, v) in &self.values {
            values.insert(k.clone(), fish_to_json(v));
        }
        serde_json::json!({
            "object_id": self.object_id,
            "entity": self.entity,
            "values": values,
            "rev": self.rev,
            "updated_at": self.updated_at.to_rfc3339(),
            "deleted": self.deleted,
        })
    }

    pub fn from_json(v: &serde_json::Value) -> Option<Self> {
        let object_id = v.get("object_id")?.as_str()?.to_string();
        let entity = v.get("entity")?.as_str()?.to_string();
        let values = v
            .get("values")?
            .as_object()?
            .iter()
            .map(|(k, x)| (k.clone(), fish_from_json(x)))
            .collect();
        let rev = v.get("rev")?.as_u64()?;
        let updated_at = v
            .get("updated_at")?
            .as_str()?
            .parse::<DateTime<Utc>>()
            .ok()?;
        let deleted = v.get("deleted")?.as_bool()?;
        Some(Self {
            object_id,
            entity,
            values,
            rev,
            updated_at,
            deleted,
        })
    }

    /// Convenience helpers for typed access
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key)?.as_i64()
    }
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_bool()
    }
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_f64()
    }
}

/// Manual FishValue <-> serde_json bridge (CoreData keeps its own
/// serde_json for SQLite blobs; FishValue no longer implements Serialize).
fn fish_to_json(v: &FishValue) -> serde_json::Value {
    match v {
        FishValue::Null => serde_json::Value::Null,
        FishValue::Bool(b) => serde_json::Value::Bool(*b),
        FishValue::Integer(i) => serde_json::Value::Number((*i).into()),
        FishValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        FishValue::String(s) => serde_json::Value::String(s.clone()),
        FishValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(fish_to_json).collect())
        }
        FishValue::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, val) in table {
                map.insert(k.clone(), fish_to_json(val));
            }
            serde_json::Value::Object(map)
        }
    }
}

fn fish_from_json(v: &serde_json::Value) -> FishValue {
    match v {
        serde_json::Value::Null => FishValue::Null,
        serde_json::Value::Bool(b) => FishValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                FishValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                FishValue::Float(f)
            } else {
                FishValue::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => FishValue::String(s.clone()),
        serde_json::Value::Array(items) => {
            FishValue::Array(items.iter().map(fish_from_json).collect())
        }
        serde_json::Value::Object(map) => {
            let mut table = IndexMap::new();
            for (k, val) in map {
                table.insert(k.clone(), fish_from_json(val));
            }
            FishValue::Table(table)
        }
    }
}

/// Entity description – schema hint (lightweight, no strict validation like full CoreData yet)
#[derive(Debug, Clone)]
pub struct EntityDescription {
    pub name: String,
    pub attributes: Vec<AttributeDescription>,
}

#[derive(Debug, Clone)]
pub struct AttributeDescription {
    pub name: String,
    pub attribute_type: AttributeType,
    pub optional: bool,
    pub default_value: Option<FishValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    Binary,
    Transformable,
}

impl EntityDescription {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: Vec::new(),
        }
    }
    pub fn with_attribute(mut self, attr: AttributeDescription) -> Self {
        self.attributes.push(attr);
        self
    }
}
