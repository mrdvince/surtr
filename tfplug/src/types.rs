//! Core type system for tfplug
//!
//! This module provides the core types used throughout the framework,
//! including Dynamic values, type definitions, and utility types.

use crate::error::{Result, TfplugError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dynamic represents Terraform values that can be of any type
/// This is the core type for all configuration and state data
/// IMPORTANT: Always use type-safe accessors instead of matching directly
#[derive(Debug, Clone, PartialEq)]
pub enum Dynamic {
    /// Explicit null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Number value (all numbers are f64 to match Terraform)
    Number(f64),
    /// String value
    String(String),
    /// List of values (ordered, allows duplicates)
    List(Vec<Dynamic>),
    /// Map of string keys to values (objects are represented as Maps)
    Map(HashMap<String, Dynamic>),
    /// Value not yet known (during planning)
    Unknown,
}

impl Serialize for Dynamic {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Dynamic::Null => serializer.serialize_unit(),
            Dynamic::Bool(b) => serializer.serialize_bool(*b),
            Dynamic::Number(n) => serializer.serialize_f64(*n),
            Dynamic::String(s) => serializer.serialize_str(s),
            Dynamic::List(l) => l.serialize(serializer),
            Dynamic::Map(m) => m.serialize(serializer),
            Dynamic::Unknown => serializer.serialize_str("__unknown__"),
        }
    }
}

impl<'de> Deserialize<'de> for Dynamic {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct DynamicVisitor;

        impl<'de> Visitor<'de> for DynamicVisitor {
            type Value = Dynamic;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid Dynamic value")
            }

            fn visit_unit<E>(self) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                Ok(Dynamic::Null)
            }

            fn visit_bool<E>(self, value: bool) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                Ok(Dynamic::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                Ok(Dynamic::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                Ok(Dynamic::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                Ok(Dynamic::Number(value))
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                if value == "__unknown__" {
                    Ok(Dynamic::Unknown)
                } else {
                    Ok(Dynamic::String(value.to_string()))
                }
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<Dynamic, E>
            where
                E: de::Error,
            {
                if value == "__unknown__" {
                    Ok(Dynamic::Unknown)
                } else {
                    Ok(Dynamic::String(value))
                }
            }

            fn visit_seq<V>(self, mut seq: V) -> std::result::Result<Dynamic, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(Dynamic::List(vec))
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Dynamic, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut hashmap = HashMap::new();
                while let Some((key, value)) = map.next_entry()? {
                    hashmap.insert(key, value);
                }
                Ok(Dynamic::Map(hashmap))
            }
        }

        deserializer.deserialize_any(DynamicVisitor)
    }
}

/// DynamicValue wraps Dynamic and provides encoding/decoding capabilities
/// This is what gets passed between Terraform and the provider
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicValue {
    pub value: Dynamic,
}

impl DynamicValue {
    pub fn new(value: Dynamic) -> Self {
        Self { value }
    }

    pub fn null() -> Self {
        Self {
            value: Dynamic::Null,
        }
    }

    pub fn unknown() -> Self {
        Self {
            value: Dynamic::Unknown,
        }
    }

    /// Encoding/decoding for wire protocol - Terraform uses msgpack by default
    pub fn encode_msgpack(&self) -> Result<Vec<u8>> {
        match &self.value {
            Dynamic::Null => Ok(vec![]),
            Dynamic::Map(map) => rmp_serde::encode::to_vec(map)
                .map_err(|e| TfplugError::EncodingError(format!("msgpack encoding failed: {}", e))),
            _ => rmp_serde::encode::to_vec(&self.value)
                .map_err(|e| TfplugError::EncodingError(format!("msgpack encoding failed: {}", e))),
        }
    }

    pub fn decode_msgpack(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Ok(Self::null());
        }

        // Try to decode as a map first (most common case from Terraform)
        match rmp_serde::decode::from_slice::<HashMap<String, Dynamic>>(data) {
            Ok(map) => Ok(Self {
                value: Dynamic::Map(map),
            }),
            Err(_) => {
                // Fall back to decoding as a Dynamic value directly
                match rmp_serde::decode::from_slice::<Dynamic>(data) {
                    Ok(value) => Ok(Self { value }),
                    Err(_) => {
                        // Try decoding as Option<HashMap> for null values
                        match rmp_serde::decode::from_slice::<Option<HashMap<String, Dynamic>>>(
                            data,
                        ) {
                            Ok(None) => Ok(Self::null()),
                            Ok(Some(map)) => Ok(Self {
                                value: Dynamic::Map(map),
                            }),
                            Err(e) => Err(TfplugError::DecodingError(format!(
                                "msgpack decoding failed: {}",
                                e
                            ))),
                        }
                    }
                }
            }
        }
    }

    pub fn encode_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.value)
            .map_err(|e| TfplugError::EncodingError(format!("json encoding failed: {}", e)))
    }

    pub fn decode_json(data: &[u8]) -> Result<Self> {
        let value = serde_json::from_slice(data)
            .map_err(|e| TfplugError::DecodingError(format!("json decoding failed: {}", e)))?;
        Ok(Self { value })
    }

    /// Type-safe accessors - ALWAYS use these instead of pattern matching
    /// These handle path navigation and type checking
    pub fn get_string(&self, path: &AttributePath) -> Result<String> {
        let value = self.navigate_path(path)?;
        match value {
            Dynamic::String(s) => Ok(s.clone()),
            _ => Err(TfplugError::TypeMismatch {
                expected: "string".to_string(),
                actual: self.type_name(value),
            }),
        }
    }

    pub fn get_number(&self, path: &AttributePath) -> Result<f64> {
        let value = self.navigate_path(path)?;
        match value {
            Dynamic::Number(n) => Ok(*n),
            _ => Err(TfplugError::TypeMismatch {
                expected: "number".to_string(),
                actual: self.type_name(value),
            }),
        }
    }

    pub fn get_bool(&self, path: &AttributePath) -> Result<bool> {
        let value = self.navigate_path(path)?;
        match value {
            Dynamic::Bool(b) => Ok(*b),
            _ => Err(TfplugError::TypeMismatch {
                expected: "bool".to_string(),
                actual: self.type_name(value),
            }),
        }
    }

    pub fn get_list(&self, path: &AttributePath) -> Result<Vec<Dynamic>> {
        let value = self.navigate_path(path)?;
        match value {
            Dynamic::List(l) => Ok(l.clone()),
            _ => Err(TfplugError::TypeMismatch {
                expected: "list".to_string(),
                actual: self.type_name(value),
            }),
        }
    }

    pub fn get_map(&self, path: &AttributePath) -> Result<HashMap<String, Dynamic>> {
        let value = self.navigate_path(path)?;
        match value {
            Dynamic::Map(m) => Ok(m.clone()),
            _ => Err(TfplugError::TypeMismatch {
                expected: "map".to_string(),
                actual: self.type_name(value),
            }),
        }
    }

    /// Type-safe setters - Use for building state/config objects
    pub fn set_string(&mut self, path: &AttributePath, value: String) -> Result<()> {
        self.set_value(path, Dynamic::String(value))
    }

    pub fn set_number(&mut self, path: &AttributePath, value: f64) -> Result<()> {
        self.set_value(path, Dynamic::Number(value))
    }

    pub fn set_bool(&mut self, path: &AttributePath, value: bool) -> Result<()> {
        self.set_value(path, Dynamic::Bool(value))
    }

    pub fn set_list(&mut self, path: &AttributePath, value: Vec<Dynamic>) -> Result<()> {
        self.set_value(path, Dynamic::List(value))
    }

    pub fn set_map(&mut self, path: &AttributePath, value: HashMap<String, Dynamic>) -> Result<()> {
        self.set_value(path, Dynamic::Map(value))
    }

    /// Helpers for handling unknown values during planning
    pub fn is_null(&self) -> bool {
        matches!(self.value, Dynamic::Null)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self.value, Dynamic::Unknown)
    }

    /// Mark computed values as unknown during planning
    pub fn mark_unknown(&mut self, path: &AttributePath) -> Result<()> {
        self.set_value(path, Dynamic::Unknown)
    }

    // Private helper methods
    fn navigate_path<'a>(&'a self, path: &AttributePath) -> Result<&'a Dynamic> {
        let mut current = &self.value;

        for step in &path.steps {
            current = match (current, step) {
                (Dynamic::Map(m), AttributePathStep::AttributeName(name)) => {
                    m.get(name).ok_or_else(|| {
                        TfplugError::Custom(format!("attribute '{}' not found", name))
                    })?
                }
                (Dynamic::List(l), AttributePathStep::ElementKeyInt(idx)) => {
                    let idx = *idx as usize;
                    l.get(idx).ok_or_else(|| {
                        TfplugError::Custom(format!("list index {} out of bounds", idx))
                    })?
                }
                _ => return Err(TfplugError::Custom("invalid path navigation".to_string())),
            };
        }

        Ok(current)
    }

    fn set_value(&mut self, path: &AttributePath, new_value: Dynamic) -> Result<()> {
        if path.steps.is_empty() {
            self.value = new_value;
            return Ok(());
        }

        // For non-empty paths, ensure we have a map at the root
        if !matches!(self.value, Dynamic::Map(_)) {
            self.value = Dynamic::Map(HashMap::new());
        }

        let mut current = &mut self.value;
        let last_idx = path.steps.len() - 1;

        for (idx, step) in path.steps.iter().enumerate() {
            if idx == last_idx {
                // Set the final value
                match (current, step) {
                    (Dynamic::Map(m), AttributePathStep::AttributeName(name)) => {
                        m.insert(name.clone(), new_value);
                        return Ok(());
                    }
                    (Dynamic::List(l), AttributePathStep::ElementKeyInt(idx)) => {
                        let idx = *idx as usize;
                        if idx < l.len() {
                            l[idx] = new_value;
                            return Ok(());
                        }
                        return Err(TfplugError::Custom(format!(
                            "list index {} out of bounds",
                            idx
                        )));
                    }
                    _ => return Err(TfplugError::Custom("invalid path navigation".to_string())),
                }
            } else {
                // Navigate to the next level
                current = match (current, step) {
                    (Dynamic::Map(m), AttributePathStep::AttributeName(name)) => {
                        m.entry(name.clone()).or_insert_with(|| {
                            // Determine what to insert based on next step
                            if let Some(next_step) = path.steps.get(idx + 1) {
                                match next_step {
                                    AttributePathStep::AttributeName(_) => {
                                        Dynamic::Map(HashMap::new())
                                    }
                                    AttributePathStep::ElementKeyInt(_) => {
                                        Dynamic::List(Vec::new())
                                    }
                                    AttributePathStep::ElementKeyString(_) => {
                                        Dynamic::Map(HashMap::new())
                                    }
                                }
                            } else {
                                Dynamic::Null
                            }
                        })
                    }
                    (Dynamic::List(l), AttributePathStep::ElementKeyInt(idx)) => {
                        let idx = *idx as usize;
                        if idx >= l.len() {
                            return Err(TfplugError::Custom(format!(
                                "list index {} out of bounds",
                                idx
                            )));
                        }
                        &mut l[idx]
                    }
                    _ => return Err(TfplugError::Custom("invalid path navigation".to_string())),
                };
            }
        }

        Err(TfplugError::Custom("failed to set value".to_string()))
    }

    fn type_name(&self, value: &Dynamic) -> String {
        match value {
            Dynamic::Null => "null".to_string(),
            Dynamic::Bool(_) => "bool".to_string(),
            Dynamic::Number(_) => "number".to_string(),
            Dynamic::String(_) => "string".to_string(),
            Dynamic::List(_) => "list".to_string(),
            Dynamic::Map(_) => "map".to_string(),
            Dynamic::Unknown => "unknown".to_string(),
        }
    }
}

/// AttributePath represents a path to an attribute within a DynamicValue
#[derive(Debug, Clone, PartialEq)]
pub struct AttributePath {
    pub steps: Vec<AttributePathStep>,
}

impl AttributePath {
    pub fn new(name: &str) -> Self {
        Self {
            steps: vec![AttributePathStep::AttributeName(name.to_string())],
        }
    }

    pub fn root() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn attribute(mut self, name: &str) -> Self {
        self.steps
            .push(AttributePathStep::AttributeName(name.to_string()));
        self
    }

    pub fn index(mut self, idx: i64) -> Self {
        self.steps.push(AttributePathStep::ElementKeyInt(idx));
        self
    }

    pub fn key(mut self, key: &str) -> Self {
        self.steps
            .push(AttributePathStep::ElementKeyString(key.to_string()));
        self
    }
}

/// Individual step in an AttributePath
#[derive(Debug, Clone, PartialEq)]
pub enum AttributePathStep {
    /// Access attribute by name in object/map
    AttributeName(String),
    /// Access element by string key (for maps)
    ElementKeyString(String),
    /// Access element by integer index (for lists)
    ElementKeyInt(i64),
}

/// Private state management - Provider-specific data not visible to users
/// IMPORTANT: This replaces raw Vec<u8> in all APIs
/// The framework handles msgpack encoding/decoding transparently
/// Reference: terraform-plugin-framework/internal/privatestate/data.go
#[derive(Debug, Clone)]
pub struct PrivateStateData {
    data: HashMap<String, Vec<u8>>,
}

impl PrivateStateData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get_key(&self, key: &str) -> Option<&[u8]> {
        self.data.get(key).map(|v| v.as_slice())
    }

    pub fn set_key(&mut self, key: &str, value: Vec<u8>) {
        self.data.insert(key.to_string(), value);
    }

    pub fn remove_key(&mut self, key: &str) {
        self.data.remove(key);
    }

    /// Encoding uses msgpack like DynamicValue for consistency
    /// Reference: HashiCorp's framework uses structured private state
    /// Source: terraform-plugin-framework/internal/privatestate/data.go
    pub fn encode(&self) -> Result<Vec<u8>> {
        rmp_serde::encode::to_vec(&self.data).map_err(|e| {
            TfplugError::EncodingError(format!("private state encoding failed: {}", e))
        })
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        let data = rmp_serde::decode::from_slice(data).map_err(|e| {
            TfplugError::DecodingError(format!("private state decoding failed: {}", e))
        })?;
        Ok(Self { data })
    }
}

impl Default for PrivateStateData {
    fn default() -> Self {
        Self::new()
    }
}

/// RawState holds the stored state for a resource to be upgraded
#[derive(Debug, Clone)]
pub struct RawState {
    pub json: Option<Vec<u8>>,
    pub flatmap: Option<HashMap<String, String>>,
}

/// Diagnostic represents a warning or error from the provider
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub summary: String,
    pub detail: String,
    pub attribute: Option<AttributePath>,
}

impl Diagnostic {
    pub fn error(summary: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            summary: summary.into(),
            detail: detail.into(),
            attribute: None,
        }
    }

    pub fn warning(summary: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            summary: summary.into(),
            detail: detail.into(),
            attribute: None,
        }
    }

    pub fn with_attribute(mut self, path: AttributePath) -> Self {
        self.attribute = Some(path);
        self
    }
}

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticSeverity {
    Invalid,
    Error,
    Warning,
}

/// ServerCapabilities indicates provider capabilities
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub plan_destroy: bool,
    pub get_provider_schema_optional: bool,
    pub move_resource_state: bool,
}

/// ClientCapabilities indicates Terraform client capabilities
#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    pub deferral_allowed: bool,
    pub write_only_attributes_allowed: bool,
}

/// Deferred indicates a deferred change
#[derive(Debug, Clone)]
pub struct Deferred {
    pub reason: DeferredReason,
}

/// Reason for deferring a change
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeferredReason {
    Unknown,
    ResourceConfigUnknown,
    ProviderConfigUnknown,
    AbsentPrereq,
}

/// ResourceIdentitySchema represents the structure of resource identity
#[derive(Debug, Clone)]
pub struct ResourceIdentitySchema {
    pub version: i64,
    pub identity_attributes: Vec<IdentityAttribute>,
}

/// Individual attribute in resource identity
#[derive(Debug, Clone)]
pub struct IdentityAttribute {
    pub name: String,
    pub type_: Vec<u8>, // Encoded type
    pub required_for_import: bool,
    pub optional_for_import: bool,
    pub description: String,
}

/// ResourceIdentityData contains actual identity data
#[derive(Debug, Clone)]
pub struct ResourceIdentityData {
    pub identity_data: DynamicValue,
}

/// FunctionError represents an error from a function call
#[derive(Debug, Clone)]
pub struct FunctionError {
    pub text: String,
    pub function_argument: Option<i64>,
}

/// Config represents configuration values
pub type Config = DynamicValue;

/// State represents resource state values
pub type State = DynamicValue;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_value_string_access() {
        let mut dv = DynamicValue::new(Dynamic::Map(HashMap::new()));
        dv.set_string(&AttributePath::new("name"), "test".to_string())
            .unwrap();

        let result = dv.get_string(&AttributePath::new("name")).unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn dynamic_value_nested_access() {
        let mut dv = DynamicValue::new(Dynamic::Map(HashMap::new()));
        let path = AttributePath::new("config").attribute("endpoint");
        dv.set_string(&path, "https://example.com".to_string())
            .unwrap();

        let result = dv.get_string(&path).unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn private_state_encoding() {
        let mut ps = PrivateStateData::new();
        ps.set_key("etag", b"12345".to_vec());

        let encoded = ps.encode().unwrap();
        let decoded = PrivateStateData::decode(&encoded).unwrap();

        assert_eq!(decoded.get_key("etag"), Some(&b"12345"[..]));
    }
}
