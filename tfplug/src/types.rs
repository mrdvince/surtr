use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub values: HashMap<String, Dynamic>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn require_string(&self, key: &str) -> crate::Result<String> {
        self.values
            .get(key)
            .and_then(|v| v.as_string())
            .cloned()
            .ok_or_else(|| format!("{} is required and must be a string", key).into())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.values.get(key).and_then(|v| v.as_string()).cloned()
    }

    pub fn require_bool(&self, key: &str) -> crate::Result<bool> {
        self.values
            .get(key)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| format!("{} is required and must be a boolean", key).into())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| v.as_bool())
    }

    pub fn require_number(&self, key: &str) -> crate::Result<f64> {
        self.values
            .get(key)
            .and_then(|v| v.as_number())
            .ok_or_else(|| format!("{} is required and must be a number", key).into())
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.values.get(key).and_then(|v| v.as_number())
    }

    pub fn get_list(&self, key: &str) -> Option<&Vec<Dynamic>> {
        self.values.get(key).and_then(|v| match v {
            Dynamic::List(list) => Some(list),
            _ => None,
        })
    }

    pub fn get_map(&self, key: &str) -> Option<&HashMap<String, Dynamic>> {
        self.values.get(key).and_then(|v| match v {
            Dynamic::Map(map) => Some(map),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub values: HashMap<String, Dynamic>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_config(config: Config) -> Self {
        Self {
            values: config.values,
        }
    }

    pub fn set_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values
            .insert(key.into(), Dynamic::String(value.into()));
    }

    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.values.insert(key.into(), Dynamic::Bool(value));
    }

    pub fn set_number(&mut self, key: impl Into<String>, value: f64) {
        self.values.insert(key.into(), Dynamic::Number(value));
    }

    pub fn set_list(&mut self, key: impl Into<String>, value: Vec<Dynamic>) {
        self.values.insert(key.into(), Dynamic::List(value));
    }

    pub fn set_map(&mut self, key: impl Into<String>, value: HashMap<String, Dynamic>) {
        self.values.insert(key.into(), Dynamic::Map(value));
    }

    pub fn require_string(&self, key: &str) -> crate::Result<String> {
        self.values
            .get(key)
            .and_then(|v| v.as_string())
            .cloned()
            .ok_or_else(|| format!("{} is required and must be a string", key).into())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.values.get(key).and_then(|v| v.as_string()).cloned()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.values.get(key).and_then(|v| v.as_number())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Dynamic {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<Dynamic>),
    Map(HashMap<String, Dynamic>),
    Unknown,
}

#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub summary: String,
    pub detail: Option<String>,
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Dynamic {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Dynamic::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Dynamic::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Dynamic::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Dynamic>> {
        match self {
            Dynamic::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, Dynamic>> {
        match self {
            Dynamic::Map(map) => Some(map),
            _ => None,
        }
    }
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_error(&mut self, summary: impl Into<String>, detail: Option<impl Into<String>>) {
        self.errors.push(Diagnostic {
            severity: Severity::Error,
            summary: summary.into(),
            detail: detail.map(Into::into),
            attribute: None,
        });
    }

    pub fn add_warning(&mut self, summary: impl Into<String>, detail: Option<impl Into<String>>) {
        self.warnings.push(Diagnostic {
            severity: Severity::Warning,
            summary: summary.into(),
            detail: detail.map(Into::into),
            attribute: None,
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn config_extraction_helpers_work() {
        let mut config = Config::new();
        config
            .values
            .insert("name".to_string(), Dynamic::String("test".to_string()));
        config
            .values
            .insert("enabled".to_string(), Dynamic::Bool(true));
        config
            .values
            .insert("count".to_string(), Dynamic::Number(42.0));
        config.values.insert(
            "tags".to_string(),
            Dynamic::List(vec![Dynamic::String("tag1".to_string())]),
        );
        let mut map = HashMap::new();
        map.insert("key".to_string(), Dynamic::String("value".to_string()));
        config
            .values
            .insert("metadata".to_string(), Dynamic::Map(map));

        assert_eq!(config.require_string("name").unwrap(), "test");
        assert_eq!(config.get_string("name").unwrap(), "test");
        assert!(config.get_string("missing").is_none());

        assert!(config.require_bool("enabled").unwrap());
        assert_eq!(config.get_bool("enabled"), Some(true));
        assert_eq!(config.get_bool("missing"), None);

        assert_eq!(config.require_number("count").unwrap(), 42.0);
        assert_eq!(config.get_number("count"), Some(42.0));
        assert_eq!(config.get_number("missing"), None);

        let list = config.get_list("tags").unwrap();
        assert_eq!(list.len(), 1);

        let map = config.get_map("metadata").unwrap();
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn config_require_methods_return_errors_for_missing_fields() {
        let config = Config::new();

        assert!(config.require_string("missing").is_err());
        assert!(config.require_bool("missing").is_err());
        assert!(config.require_number("missing").is_err());
    }

    #[test]
    fn config_require_methods_return_errors_for_wrong_types() {
        let mut config = Config::new();
        config
            .values
            .insert("field".to_string(), Dynamic::Bool(true));

        assert!(config.require_string("field").is_err());
        assert!(config.require_number("field").is_err());
    }

    #[test]
    fn state_from_config_copies_values() {
        let mut config = Config::new();
        config
            .values
            .insert("name".to_string(), Dynamic::String("test".to_string()));
        config
            .values
            .insert("enabled".to_string(), Dynamic::Bool(true));

        let state = State::from_config(config.clone());

        assert_eq!(state.values, config.values);
    }

    #[test]
    fn state_setter_methods_work() {
        let mut state = State::new();

        state.set_string("name", "test");
        assert_eq!(state.get_string("name").unwrap(), "test");

        state.set_bool("enabled", true);
        assert_eq!(state.get_bool("enabled"), Some(true));

        state.set_number("count", 42.0);
        assert_eq!(state.get_number("count"), Some(42.0));

        state.set_list("tags", vec![Dynamic::String("tag1".to_string())]);
        assert_eq!(
            state.values.get("tags").unwrap().as_list().unwrap().len(),
            1
        );

        let mut map = HashMap::new();
        map.insert("key".to_string(), Dynamic::String("value".to_string()));
        state.set_map("metadata", map);
        assert_eq!(
            state
                .values
                .get("metadata")
                .unwrap()
                .as_map()
                .unwrap()
                .len(),
            1
        );
    }
}
