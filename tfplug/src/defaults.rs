//! Default value providers for attributes
//!
//! This module provides built-in default value providers that can be used to set
//! default values for optional attributes when they are not provided in configuration.
//!
//! Default providers are evaluated during the planning phase when an attribute is not
//! set in the configuration. They differ from plan modifiers in that they only run
//! when the value is absent, not when it's explicitly set to null.
//!
//! # Examples
//!
//! ```no_run
//! use tfplug::schema::{AttributeBuilder, AttributeType};
//! use tfplug::defaults::{StaticDefault, EnvDefault};
//! use tfplug::types::Dynamic;
//!
//! // Static default value
//! let timeout = AttributeBuilder::new("timeout", AttributeType::Number)
//!     .optional()
//!     .default(StaticDefault::create(Dynamic::Number(30.0)))
//!     .build();
//!
//! // Default from environment variable
//! let region = AttributeBuilder::new("region", AttributeType::String)
//!     .optional()
//!     .default(EnvDefault::create("AWS_DEFAULT_REGION", "us-east-1"))
//!     .build();
//! ```

use crate::schema::{Default, DefaultRequest, DefaultResponse};
use crate::types::{Dynamic, DynamicValue};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// StaticDefault provides a static default value
pub struct StaticDefault {
    value: Dynamic,
}

impl StaticDefault {
    /// Create a new static default provider with the given value
    pub fn create(value: Dynamic) -> Box<dyn Default> {
        Box::new(Self { value })
    }

    /// Create a static string default
    pub fn string(value: &str) -> Box<dyn Default> {
        Box::new(Self {
            value: Dynamic::String(value.to_string()),
        })
    }

    /// Create a static number default
    pub fn number(value: f64) -> Box<dyn Default> {
        Box::new(Self {
            value: Dynamic::Number(value),
        })
    }

    /// Create a static boolean default
    pub fn bool(value: bool) -> Box<dyn Default> {
        Box::new(Self {
            value: Dynamic::Bool(value),
        })
    }

    /// Create a static list default
    pub fn list(values: Vec<Dynamic>) -> Box<dyn Default> {
        Box::new(Self {
            value: Dynamic::List(values),
        })
    }
}

impl Default for StaticDefault {
    fn description(&self) -> String {
        format!("static default value: {:?}", self.value)
    }

    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        DefaultResponse {
            value: DynamicValue::new(self.value.clone()),
        }
    }
}

/// EnvDefault gets the default value from an environment variable
pub struct EnvDefault {
    env_var: String,
    fallback: Option<String>,
}

impl EnvDefault {
    /// Create a new environment variable default provider
    pub fn create(env_var: &str, fallback: &str) -> Box<dyn Default> {
        Box::new(Self {
            env_var: env_var.to_string(),
            fallback: Some(fallback.to_string()),
        })
    }

    /// Create an environment variable default without a fallback
    pub fn create_required(env_var: &str) -> Box<dyn Default> {
        Box::new(Self {
            env_var: env_var.to_string(),
            fallback: None,
        })
    }
}

impl Default for EnvDefault {
    fn description(&self) -> String {
        match &self.fallback {
            Some(fallback) => format!(
                "default from environment variable {} (fallback: {})",
                self.env_var, fallback
            ),
            None => format!("default from environment variable {}", self.env_var),
        }
    }

    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        let value = match env::var(&self.env_var) {
            Ok(val) => Dynamic::String(val),
            Err(_) => match &self.fallback {
                Some(fallback) => Dynamic::String(fallback.clone()),
                None => Dynamic::Null,
            },
        };

        DefaultResponse {
            value: DynamicValue::new(value),
        }
    }
}

/// CurrentTimestampDefault provides the current timestamp as a default value
pub struct CurrentTimestampDefault {
    format: TimestampFormat,
}

/// Format for timestamp output
#[derive(Debug, Clone, Copy)]
pub enum TimestampFormat {
    /// Unix timestamp in seconds
    UnixSeconds,
    /// Unix timestamp in milliseconds
    UnixMilliseconds,
    /// ISO 8601 format (e.g., "2023-04-15T10:30:00Z")
    Iso8601,
    /// RFC 3339 format (e.g., "2023-04-15T10:30:00+00:00")
    Rfc3339,
}

impl CurrentTimestampDefault {
    /// Create a timestamp default with Unix seconds format
    pub fn unix_seconds() -> Box<dyn Default> {
        Box::new(Self {
            format: TimestampFormat::UnixSeconds,
        })
    }

    /// Create a timestamp default with Unix milliseconds format
    pub fn unix_milliseconds() -> Box<dyn Default> {
        Box::new(Self {
            format: TimestampFormat::UnixMilliseconds,
        })
    }

    /// Create a timestamp default with ISO 8601 format
    pub fn iso8601() -> Box<dyn Default> {
        Box::new(Self {
            format: TimestampFormat::Iso8601,
        })
    }

    /// Create a timestamp default with RFC 3339 format
    pub fn rfc3339() -> Box<dyn Default> {
        Box::new(Self {
            format: TimestampFormat::Rfc3339,
        })
    }
}

impl Default for CurrentTimestampDefault {
    fn description(&self) -> String {
        let format_desc = match self.format {
            TimestampFormat::UnixSeconds => "Unix seconds",
            TimestampFormat::UnixMilliseconds => "Unix milliseconds",
            TimestampFormat::Iso8601 => "ISO 8601",
            TimestampFormat::Rfc3339 => "RFC 3339",
        };
        format!("current timestamp in {} format", format_desc)
    }

    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        let now = SystemTime::now();
        let value = match self.format {
            TimestampFormat::UnixSeconds => {
                let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
                Dynamic::Number(duration.as_secs() as f64)
            }
            TimestampFormat::UnixMilliseconds => {
                let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
                Dynamic::Number(duration.as_millis() as f64)
            }
            TimestampFormat::Iso8601 => {
                let datetime = chrono::DateTime::<chrono::Utc>::from(now);
                Dynamic::String(datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            }
            TimestampFormat::Rfc3339 => {
                let datetime = chrono::DateTime::<chrono::Utc>::from(now);
                Dynamic::String(datetime.to_rfc3339())
            }
        };

        DefaultResponse {
            value: DynamicValue::new(value),
        }
    }
}

/// UuidDefault generates a UUID as the default value
pub struct UuidDefault {
    format: UuidFormat,
}

/// Format for UUID output
#[derive(Debug, Clone, Copy)]
pub enum UuidFormat {
    /// Standard hyphenated format (e.g., "550e8400-e29b-41d4-a716-446655440000")
    Hyphenated,
    /// Simple format without hyphens (e.g., "550e8400e29b41d4a716446655440000")
    Simple,
    /// URN format (e.g., "urn:uuid:550e8400-e29b-41d4-a716-446655440000")
    Urn,
}

impl UuidDefault {
    /// Create a UUID default with standard hyphenated format
    pub fn hyphenated() -> Box<dyn Default> {
        Box::new(Self {
            format: UuidFormat::Hyphenated,
        })
    }

    /// Create a UUID default with simple format (no hyphens)
    pub fn simple() -> Box<dyn Default> {
        Box::new(Self {
            format: UuidFormat::Simple,
        })
    }

    /// Create a UUID default with URN format
    pub fn urn() -> Box<dyn Default> {
        Box::new(Self {
            format: UuidFormat::Urn,
        })
    }
}

impl Default for UuidDefault {
    fn description(&self) -> String {
        let format_desc = match self.format {
            UuidFormat::Hyphenated => "hyphenated",
            UuidFormat::Simple => "simple",
            UuidFormat::Urn => "URN",
        };
        format!("generated UUID in {} format", format_desc)
    }

    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        let uuid = Uuid::new_v4();
        let value = match self.format {
            UuidFormat::Hyphenated => Dynamic::String(uuid.to_string()),
            UuidFormat::Simple => Dynamic::String(uuid.simple().to_string()),
            UuidFormat::Urn => Dynamic::String(uuid.urn().to_string()),
        };

        DefaultResponse {
            value: DynamicValue::new(value),
        }
    }
}

/// ConditionalDefault provides a default value based on a condition
pub struct ConditionalDefault<F>
where
    F: Fn(&DefaultRequest) -> Dynamic + Send + Sync,
{
    condition_fn: F,
    description: String,
}

impl<F> ConditionalDefault<F>
where
    F: Fn(&DefaultRequest) -> Dynamic + Send + Sync + 'static,
{
    /// Create a conditional default with a custom function
    pub fn create(description: &str, condition_fn: F) -> Box<dyn Default> {
        Box::new(Self {
            condition_fn,
            description: description.to_string(),
        })
    }
}

impl<F> Default for ConditionalDefault<F>
where
    F: Fn(&DefaultRequest) -> Dynamic + Send + Sync,
{
    fn description(&self) -> String {
        format!("conditional default: {}", self.description)
    }

    fn default_value(&self, request: DefaultRequest) -> DefaultResponse {
        let value = (self.condition_fn)(&request);
        DefaultResponse {
            value: DynamicValue::new(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AttributePath;
    use std::collections::HashMap;

    #[test]
    fn static_default_string() {
        let default = StaticDefault::string("default-value");
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        assert_eq!(
            response.value.value,
            Dynamic::String("default-value".to_string())
        );
    }

    #[test]
    fn static_default_number() {
        let default = StaticDefault::number(42.0);
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        assert_eq!(response.value.value, Dynamic::Number(42.0));
    }

    #[test]
    fn static_default_bool() {
        let default = StaticDefault::bool(true);
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        assert_eq!(response.value.value, Dynamic::Bool(true));
    }

    #[test]
    fn static_default_list() {
        let default = StaticDefault::list(vec![
            Dynamic::String("item1".to_string()),
            Dynamic::String("item2".to_string()),
        ]);
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        if let Dynamic::List(items) = response.value.value {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Dynamic::String("item1".to_string()));
            assert_eq!(items[1], Dynamic::String("item2".to_string()));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn env_default_with_fallback() {
        // Use a non-existent env var to test fallback
        let default = EnvDefault::create("TFPLUG_TEST_NONEXISTENT", "fallback-value");
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        assert_eq!(
            response.value.value,
            Dynamic::String("fallback-value".to_string())
        );
    }

    #[test]
    fn env_default_with_value() {
        // Set a temporary env var
        env::set_var("TFPLUG_TEST_VAR", "env-value");
        let default = EnvDefault::create("TFPLUG_TEST_VAR", "fallback");
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        assert_eq!(
            response.value.value,
            Dynamic::String("env-value".to_string())
        );

        // Clean up
        env::remove_var("TFPLUG_TEST_VAR");
    }

    #[test]
    fn env_default_required_missing() {
        let default = EnvDefault::create_required("TFPLUG_TEST_MISSING");
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        assert_eq!(response.value.value, Dynamic::Null);
    }

    #[test]
    fn timestamp_unix_seconds() {
        let default = CurrentTimestampDefault::unix_seconds();
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        if let Dynamic::Number(timestamp) = response.value.value {
            // Check it's a reasonable Unix timestamp (after year 2020)
            assert!(timestamp > 1577836800.0);
            // Check it's not in the future (with some buffer)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as f64;
            assert!(timestamp <= now + 10.0);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn timestamp_iso8601() {
        let default = CurrentTimestampDefault::iso8601();
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        if let Dynamic::String(timestamp) = response.value.value {
            // Check format matches ISO 8601
            assert!(timestamp.contains('T'));
            assert!(timestamp.ends_with('Z'));
            assert_eq!(timestamp.len(), 20); // "2023-04-15T10:30:00Z"
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn uuid_hyphenated() {
        let default = UuidDefault::hyphenated();
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        if let Dynamic::String(uuid) = response.value.value {
            // Check format: 8-4-4-4-12
            let parts: Vec<&str> = uuid.split('-').collect();
            assert_eq!(parts.len(), 5);
            assert_eq!(parts[0].len(), 8);
            assert_eq!(parts[1].len(), 4);
            assert_eq!(parts[2].len(), 4);
            assert_eq!(parts[3].len(), 4);
            assert_eq!(parts[4].len(), 12);
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn uuid_simple() {
        let default = UuidDefault::simple();
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        if let Dynamic::String(uuid) = response.value.value {
            // Check it's 32 hex characters with no hyphens
            assert_eq!(uuid.len(), 32);
            assert!(!uuid.contains('-'));
            assert!(uuid.chars().all(|c| c.is_ascii_hexdigit()));
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn uuid_urn() {
        let default = UuidDefault::urn();
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });

        if let Dynamic::String(uuid) = response.value.value {
            assert!(uuid.starts_with("urn:uuid:"));
            assert_eq!(uuid.len(), 45); // "urn:uuid:" (9) + standard UUID (36)
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn conditional_default() {
        let default = ConditionalDefault::create("based on path", |request| {
            if request.path.steps.is_empty() {
                Dynamic::String("root".to_string())
            } else {
                Dynamic::String("nested".to_string())
            }
        });

        // Test with root path
        let response = default.default_value(DefaultRequest {
            path: AttributePath::root(),
        });
        assert_eq!(response.value.value, Dynamic::String("root".to_string()));

        // Test with nested path
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("test"),
        });
        assert_eq!(response.value.value, Dynamic::String("nested".to_string()));
    }

    #[test]
    fn complex_static_default() {
        let mut map = HashMap::new();
        map.insert("host".to_string(), Dynamic::String("localhost".to_string()));
        map.insert("port".to_string(), Dynamic::Number(8080.0));
        map.insert("ssl".to_string(), Dynamic::Bool(false));

        let default = StaticDefault::create(Dynamic::Map(map));
        let response = default.default_value(DefaultRequest {
            path: AttributePath::new("config"),
        });

        if let Dynamic::Map(config) = response.value.value {
            assert_eq!(
                config.get("host"),
                Some(&Dynamic::String("localhost".to_string()))
            );
            assert_eq!(config.get("port"), Some(&Dynamic::Number(8080.0)));
            assert_eq!(config.get("ssl"), Some(&Dynamic::Bool(false)));
        } else {
            panic!("Expected map");
        }
    }
}
