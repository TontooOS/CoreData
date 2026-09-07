//! FetchRequest – NSPredicate/NSSortDescriptor inspired

use crate::entity::ManagedObject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredicateOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Contains,
    BeginsWith,
    EndsWith,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predicate {
    pub key: String,
    pub op: PredicateOperator,
    pub value: String,
    /// case-insensitive for string ops
    pub case_insensitive: bool,
}

impl Predicate {
    pub fn new(key: impl Into<String>, op: PredicateOperator, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            op,
            value: value.into(),
            case_insensitive: false,
        }
    }

    pub fn case_insensitive(mut self, v: bool) -> Self {
        self.case_insensitive = v;
        self
    }

    /// Simple format parser: "key == value", "age > 30", "name CONTAINS 'foo'"
    pub fn from_format(format: &str) -> crate::error::Result<Self> {
        // Very small parser for convenience – real apps should use Predicate::new
        let tokens: Vec<&str> = format.split_whitespace().collect();
        if tokens.len() < 3 {
            return Err(crate::error::CoreDataError::custom(format!("invalid predicate format: {}", format)));
        }
        let key = tokens[0].to_string();
        let op_str = tokens[1];
        let value = tokens[2..].join(" ").trim_matches('\'').trim_matches('"').to_string();
        let op = match op_str {
            "==" | "=" | "Equal" => PredicateOperator::Equal,
            "!=" | "NotEqual" => PredicateOperator::NotEqual,
            "<" => PredicateOperator::LessThan,
            "<=" => PredicateOperator::LessThanOrEqual,
            ">" => PredicateOperator::GreaterThan,
            ">=" => PredicateOperator::GreaterThanOrEqual,
            "CONTAINS" | "Contains" | "contains" => PredicateOperator::Contains,
            "BEGINSWITH" => PredicateOperator::BeginsWith,
            "ENDSWITH" => PredicateOperator::EndsWith,
            "IN" => PredicateOperator::In,
            _ => return Err(crate::error::CoreDataError::custom(format!("unknown operator {}", op_str))),
        };
        Ok(Self::new(key, op, value))
    }

    pub fn evaluate(&self, object: &ManagedObject) -> bool {
        let Some(val) = object.get(&self.key) else {
            return false;
        };
        // Normalize for string compare
        let left_str = match val {
            fishfile::FishValue::String(s) => s.clone(),
            fishfile::FishValue::Integer(i) => i.to_string(),
            fishfile::FishValue::Float(f) => f.to_string(),
            fishfile::FishValue::Bool(b) => b.to_string(),
            _ => format!("{}", val),
        };
        let right = &self.value;
        let (l, r) = if self.case_insensitive {
            (left_str.to_lowercase(), right.to_lowercase())
        } else {
            (left_str, right.clone())
        };

        // Try numeric compare first if both parse
        let l_num = l.parse::<f64>().ok();
        let r_num = r.parse::<f64>().ok();

        match self.op {
            PredicateOperator::Equal => l == r,
            PredicateOperator::NotEqual => l != r,
            PredicateOperator::LessThan => {
                if let (Some(a), Some(b)) = (l_num, r_num) {
                    a < b
                } else {
                    l < r
                }
            }
            PredicateOperator::LessThanOrEqual => {
                if let (Some(a), Some(b)) = (l_num, r_num) {
                    a <= b
                } else {
                    l <= r
                }
            }
            PredicateOperator::GreaterThan => {
                if let (Some(a), Some(b)) = (l_num, r_num) {
                    a > b
                } else {
                    l > r
                }
            }
            PredicateOperator::GreaterThanOrEqual => {
                if let (Some(a), Some(b)) = (l_num, r_num) {
                    a >= b
                } else {
                    l >= r
                }
            }
            PredicateOperator::Contains => l.contains(&r as &str),
            PredicateOperator::BeginsWith => l.starts_with(&r as &str),
            PredicateOperator::EndsWith => l.ends_with(&r as &str),
            PredicateOperator::In => {
                let parts: Vec<String> = r.split(',').map(|s| s.trim().to_string()).collect();
                parts.contains(&l)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortDescriptor {
    pub key: String,
    pub ascending: bool,
    pub case_insensitive: bool,
}

impl SortDescriptor {
    pub fn new(key: impl Into<String>, ascending: bool) -> Self {
        Self {
            key: key.into(),
            ascending,
            case_insensitive: false,
        }
    }
    pub fn case_insensitive(mut self, v: bool) -> Self {
        self.case_insensitive = v;
        self
    }
}

#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub entity: String,
    pub predicate: Option<Predicate>,
    pub predicates: Vec<Predicate>, // and-combined
    pub sort_descriptors: Vec<SortDescriptor>,
    pub fetch_limit: Option<usize>,
    pub fetch_offset: usize,
    pub include_deleted: bool,
}

impl FetchRequest {
    pub fn new(entity: impl Into<String>) -> Self {
        Self {
            entity: entity.into(),
            predicate: None,
            predicates: Vec::new(),
            sort_descriptors: Vec::new(),
            fetch_limit: None,
            fetch_offset: 0,
            include_deleted: false,
        }
    }

    pub fn predicate(mut self, p: Predicate) -> Self {
        self.predicate = Some(p);
        self
    }

    pub fn and_predicate(mut self, p: Predicate) -> Self {
        self.predicates.push(p);
        self
    }

    pub fn sorted_by(mut self, d: SortDescriptor) -> Self {
        self.sort_descriptors.push(d);
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.fetch_limit = Some(n);
        self
    }

    pub fn offset(mut self, n: usize) -> Self {
        self.fetch_offset = n;
        self
    }

    pub fn include_deleted(mut self, v: bool) -> Self {
        self.include_deleted = v;
        self
    }

    pub fn matches(&self, obj: &ManagedObject) -> bool {
        if obj.entity != self.entity {
            return false;
        }
        if !self.include_deleted && obj.deleted {
            return false;
        }
        if let Some(p) = &self.predicate {
            if !p.evaluate(obj) {
                return false;
            }
        }
        for p in &self.predicates {
            if !p.evaluate(obj) {
                return false;
            }
        }
        true
    }

    pub fn apply(&self, objects: Vec<ManagedObject>) -> Vec<ManagedObject> {
        let mut filtered: Vec<ManagedObject> = objects.into_iter().filter(|o| self.matches(o)).collect();
        if !self.sort_descriptors.is_empty() {
            filtered.sort_by(|a, b| {
                for desc in &self.sort_descriptors {
                    let av = a.get(&desc.key).map(val_to_string).unwrap_or_default();
                    let bv = b.get(&desc.key).map(val_to_string).unwrap_or_default();
                    let (avc, bvc) = if desc.case_insensitive {
                        (av.to_lowercase(), bv.to_lowercase())
                    } else {
                        (av, bv)
                    };
                    // numeric attempt
                    let ord = if let (Ok(an), Ok(bn)) = (avc.parse::<f64>(), bvc.parse::<f64>()) {
                        an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        avc.cmp(&bvc)
                    };
                    if ord != std::cmp::Ordering::Equal {
                        return if desc.ascending { ord } else { ord.reverse() };
                    }
                }
                std::cmp::Ordering::Equal
            });
        }
        let start = self.fetch_offset.min(filtered.len());
        let mut sliced = filtered.into_iter().skip(start).collect::<Vec<_>>();
        if let Some(limit) = self.fetch_limit {
            sliced.truncate(limit);
        }
        sliced
    }
}

fn val_to_string(v: &fishfile::FishValue) -> String {
    match v {
        fishfile::FishValue::String(s) => s.clone(),
        fishfile::FishValue::Integer(i) => i.to_string(),
        fishfile::FishValue::Float(f) => f.to_string(),
        fishfile::FishValue::Bool(b) => b.to_string(),
        fishfile::FishValue::Null => String::new(),
        other => format!("{}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(name: &str, age: i64) -> ManagedObject {
        let mut o = ManagedObject::new("Person");
        o.set("name", name);
        o.set("age", age);
        o
    }

    #[test]
    fn predicate_eval() {
        let o = obj("Alice", 30);
        assert!(Predicate::new("name", PredicateOperator::Equal, "Alice").evaluate(&o));
        assert!(Predicate::new("age", PredicateOperator::GreaterThan, "20").evaluate(&o));
        assert!(!Predicate::new("age", PredicateOperator::LessThan, "20").evaluate(&o));
        assert!(Predicate::new("name", PredicateOperator::Contains, "lic").evaluate(&o));
    }

    #[test]
    fn fetch_sort_limit() {
        let objs = vec![obj("Charlie", 30), obj("Alice", 25), obj("Bob", 20)];
        let req = FetchRequest::new("Person").sorted_by(SortDescriptor::new("name", true)).limit(2);
        let res = req.apply(objs);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].get_str("name"), Some("Alice"));
    }
}
