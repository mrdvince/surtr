//! Common types and utilities for Proxmox API

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct TaskId(pub String);

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub errors: Option<Vec<String>>,
    pub data: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, thiserror::Error)]
#[error("API error details: errors={errors:?}, field_errors={field_errors:?}")]
pub struct ApiErrorDetails {
    pub errors: Option<Vec<String>>,
    pub field_errors: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxmoxBool(pub bool);

impl ProxmoxBool {
    pub fn new(value: bool) -> Self {
        Self(value)
    }

    pub fn as_bool(&self) -> bool {
        self.0
    }
}

impl From<bool> for ProxmoxBool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<ProxmoxBool> for bool {
    fn from(value: ProxmoxBool) -> Self {
        value.0
    }
}

impl Serialize for ProxmoxBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProxmoxBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum BoolOrInt {
            Bool(bool),
            Int(u8),
        }

        match BoolOrInt::deserialize(deserializer)? {
            BoolOrInt::Bool(b) => Ok(ProxmoxBool(b)),
            BoolOrInt::Int(0) => Ok(ProxmoxBool(false)),
            BoolOrInt::Int(1) => Ok(ProxmoxBool(true)),
            BoolOrInt::Int(_) => Err(serde::de::Error::custom("expected 0 or 1")),
        }
    }
}

pub fn deserialize_proxmox_bool_option<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<ProxmoxBool>::deserialize(deserializer)?.map(|b| b.0))
}

pub trait ProxmoxApiResource: Sized {
    type CreateRequest: Serialize;
    type UpdateRequest: Serialize;

    fn api_path() -> &'static str;
    fn resource_path(id: &str) -> String {
        format!("{}/{}", Self::api_path(), id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ApiQueryParams {
    params: Vec<(String, String)>,
}

impl ApiQueryParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<K: Into<String>, V: ToString>(mut self, key: K, value: V) -> Self {
        self.params.push((key.into(), value.to_string()));
        self
    }

    pub fn add_optional<K: Into<String>, V: ToString>(mut self, key: K, value: Option<V>) -> Self {
        if let Some(v) = value {
            self.params.push((key.into(), v.to_string()));
        }
        self
    }

    pub fn to_query_string(&self) -> String {
        if self.params.is_empty() {
            String::new()
        } else {
            format!(
                "?{}",
                self.params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PaginationParams {
    pub start: Option<u32>,
    pub limit: Option<u32>,
}

impl PaginationParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_start(mut self, start: u32) -> Self {
        self.start = Some(start);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn to_query_params(&self) -> ApiQueryParams {
        let mut params = ApiQueryParams::new();
        params = params.add_optional("start", self.start);
        params = params.add_optional("limit", self.limit);
        params
    }
}

pub struct ApiListResponse<T> {
    pub data: Vec<T>,
    pub total: Option<u32>,
}

pub mod string_or_u64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => serializer.serialize_some(&v.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU64 {
            String(String),
            U64(u64),
        }

        match Option::<StringOrU64>::deserialize(deserializer)? {
            Some(StringOrU64::String(s)) => {
                s.parse::<u64>().map(Some).map_err(serde::de::Error::custom)
            }
            Some(StringOrU64::U64(u)) => Ok(Some(u)),
            None => Ok(None),
        }
    }
}

pub mod string_or_u32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => serializer.serialize_some(&v.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU32 {
            String(String),
            U32(u32),
        }

        match Option::<StringOrU32>::deserialize(deserializer)? {
            Some(StringOrU32::String(s)) => {
                s.parse::<u32>().map(Some).map_err(serde::de::Error::custom)
            }
            Some(StringOrU32::U32(u)) => Ok(Some(u)),
            None => Ok(None),
        }
    }
}
