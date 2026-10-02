//! ManagedObject – CoreData-like entity instance

use fishfile::FishValue;
use foundation::collections::OrderedMap;
use foundation::date::{Date, ISO8601DateFormatter};
use foundation::serialization::JsonValue;

/// A single managed object (row / document).
#[derive(Debug, Clone)]
pub struct ManagedObject {
    /// Stable object ID – UUID v4
    pub object_id: String,
    /// Entity / table name
    pub entity: String,
    /// Attributes
    pub values: OrderedMap<String, FishValue>,
    /// Revision for sync (Lamport)
    pub rev: u64,
    /// Updated at (second resolution)
    pub updated_at: Date,
    /// Soft delete marker
    pub deleted: bool,
}

impl ManagedObject {
    pub fn new(entity: impl Into<String>) -> Self {
        Self {
            object_id: foundation::uuid::new_v4_string(),
            entity: entity.into(),
            values: OrderedMap::new(),
            rev: 1,
            updated_at: Date::now(),
            deleted: false,
        }
    }

    pub fn with_id(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            object_id: id.into(),
            entity: entity.into(),
            values: OrderedMap::new(),
            rev: 1,
            updated_at: Date::now(),
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
        self.updated_at = Date::now();
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = true;
        self.bump();
    }

    /// The `updated_at` timestamp as an RFC 3339 string.
    pub fn updated_at_string(&self) -> String {
        ISO8601DateFormatter::string_from(&self.updated_at)
    }

    /// Parse an RFC 3339 timestamp. Accepts both the current
    /// `2026-01-01T12:00:00.000Z` form and the nanosecond form written by
    /// older builds (`2026-01-01T12:00:00.123456789+00:00`), so existing
    /// `.fico` and SQLite stores keep loading.
    pub fn parse_timestamp(text: &str) -> Option<Date> {
        ISO8601DateFormatter::date_from(text).ok()
    }

    /// Convert to FishValue::Table for Fico storage.
    pub fn to_fish_value(&self) -> FishValue {
        let mut t = OrderedMap::new();
        t.insert("object_id".to_string(), FishValue::String(self.object_id.clone()));
        t.insert("entity".to_string(), FishValue::String(self.entity.clone()));
        t.insert("rev".to_string(), FishValue::Integer(self.rev as i64));
        t.insert("updated_at".to_string(), FishValue::String(self.updated_at_string()));
        t.insert("deleted".to_string(), FishValue::Bool(self.deleted));
        for (k, v) in &self.values {
            t.insert(k.clone(), v.clone());
        }
        FishValue::Table(t)
    }

    pub fn from_fish_value(entity_hint: &str, id: &str, v: &FishValue) -> Option<Self> {
        let table = v.as_table()?;
        let mut values = OrderedMap::new();
        let mut rev = 1u64;
        let mut updated_at = Date::now();
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
                        if let Some(date) = Self::parse_timestamp(s) {
                            updated_at = date;
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

    /// To JSON for the SQLite blob. The shape is unchanged, so blobs written
    /// by earlier builds still decrypt and load.
    pub fn to_json(&self) -> JsonValue {
        let values: Vec<(String, JsonValue)> = self
            .values
            .iter()
            .map(|(k, v)| (k.clone(), fish_to_json(v)))
            .collect();
        JsonValue::Object(vec![
            ("object_id".to_string(), JsonValue::Str(self.object_id.clone())),
            ("entity".to_string(), JsonValue::Str(self.entity.clone())),
            ("values".to_string(), JsonValue::Object(values)),
            (
                "rev".to_string(),
                JsonValue::Integer(self.rev as i64),
            ),
            (
                "updated_at".to_string(),
                JsonValue::Str(self.updated_at_string()),
            ),
            ("deleted".to_string(), JsonValue::Bool(self.deleted)),
        ])
    }

    pub fn from_json(v: &JsonValue) -> Option<Self> {
        let object_id = v.get("object_id")?.as_str()?.to_string();
        let entity = v.get("entity")?.as_str()?.to_string();
        let mut values = OrderedMap::new();
        for (key, item) in v.get("values")?.object_entries()? {
            values.insert(key.clone(), fish_from_json(item));
        }
        let rev = v.get("rev")?.as_u64()?;
        let updated_at = Self::parse_timestamp(v.get("updated_at")?.as_str()?)?;
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

/// Manual FishValue <-> JSON bridge on top of `foundation::serialization`
/// (FishValue has no serialization impls).
fn fish_to_json(v: &FishValue) -> JsonValue {
    match v {
        FishValue::Null => JsonValue::Null,
        FishValue::Bool(b) => JsonValue::Bool(*b),
        FishValue::Integer(i) => JsonValue::Integer(*i),
        FishValue::Float(f) if f.is_finite() => JsonValue::Float(*f),
        FishValue::Float(_) => JsonValue::Null,
        FishValue::String(s) => JsonValue::Str(s.clone()),
        FishValue::Array(items) => {
            JsonValue::Array(items.iter().map(fish_to_json).collect())
        }
        FishValue::Table(table) => JsonValue::Object(
            table
                .iter()
                .map(|(k, val)| (k.clone(), fish_to_json(val)))
                .collect(),
        ),
    }
}

fn fish_from_json(v: &JsonValue) -> FishValue {
    match v {
        JsonValue::Null => FishValue::Null,
        JsonValue::Bool(b) => FishValue::Bool(*b),
        JsonValue::Integer(i) => FishValue::Integer(*i),
        JsonValue::Float(f) => FishValue::Float(*f),
        JsonValue::Str(s) => FishValue::String(s.clone()),
        JsonValue::Array(items) => FishValue::Array(items.iter().map(fish_from_json).collect()),
        JsonValue::Object(entries) => {
            let mut table = OrderedMap::new();
            for (key, value) in entries {
                table.insert(key.clone(), fish_from_json(value));
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
