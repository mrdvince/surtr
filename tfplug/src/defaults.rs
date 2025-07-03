use crate::types::Dynamic;

#[derive(Debug, Clone)]
pub struct DefaultRequest {
    pub attribute_path: String,
}

#[derive(Debug, Clone)]
pub struct DefaultResponse {
    pub value: Dynamic,
}

/// Default values are applied during planning when the configuration value is null.
/// They can only be used on attributes that are both Optional and Computed.
pub trait Default: Send + Sync {
    fn default_value(&self, request: DefaultRequest) -> DefaultResponse;

    fn description(&self) -> String {
        "default value".to_string()
    }
}

pub struct StaticBool {
    value: bool,
}

impl StaticBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl Default for StaticBool {
    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        DefaultResponse {
            value: Dynamic::Bool(self.value),
        }
    }

    fn description(&self) -> String {
        format!("defaults to {}", self.value)
    }
}

pub struct StaticString {
    value: String,
}

impl StaticString {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl Default for StaticString {
    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        DefaultResponse {
            value: Dynamic::String(self.value.clone()),
        }
    }

    fn description(&self) -> String {
        format!("defaults to \"{}\"", self.value)
    }
}

pub struct StaticNumber {
    value: f64,
}

impl StaticNumber {
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}

impl Default for StaticNumber {
    fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
        DefaultResponse {
            value: Dynamic::Number(self.value),
        }
    }

    fn description(&self) -> String {
        format!("defaults to {}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_bool_returns_configured_value() {
        let default = StaticBool::new(true);
        let request = DefaultRequest {
            attribute_path: "test.field".to_string(),
        };

        let response = default.default_value(request);

        match response.value {
            Dynamic::Bool(val) => assert!(val),
            _ => panic!("Expected Bool variant"),
        }
    }

    #[test]
    fn static_bool_description() {
        let default = StaticBool::new(false);
        assert_eq!(default.description(), "defaults to false");
    }

    #[test]
    fn static_string_returns_configured_value() {
        let default = StaticString::new("hello");
        let request = DefaultRequest {
            attribute_path: "test.field".to_string(),
        };

        let response = default.default_value(request);

        match response.value {
            Dynamic::String(val) => assert_eq!(val, "hello"),
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn static_string_description() {
        let default = StaticString::new("test");
        assert_eq!(default.description(), "defaults to \"test\"");
    }

    #[test]
    fn static_number_returns_configured_value() {
        let default = StaticNumber::new(42.5);
        let request = DefaultRequest {
            attribute_path: "test.field".to_string(),
        };

        let response = default.default_value(request);

        match response.value {
            Dynamic::Number(val) => assert_eq!(val, 42.5),
            _ => panic!("Expected Number variant"),
        }
    }

    #[test]
    fn static_number_description() {
        let default = StaticNumber::new(100.0);
        assert_eq!(default.description(), "defaults to 100");
    }

    #[test]
    fn custom_default_implementation() {
        struct EnvironmentDefault {
            env_var: String,
            fallback: String,
        }

        impl Default for EnvironmentDefault {
            fn default_value(&self, _request: DefaultRequest) -> DefaultResponse {
                let value = std::env::var(&self.env_var).unwrap_or_else(|_| self.fallback.clone());

                DefaultResponse {
                    value: Dynamic::String(value),
                }
            }

            fn description(&self) -> String {
                format!("defaults to ${} or \"{}\"", self.env_var, self.fallback)
            }
        }

        let default = EnvironmentDefault {
            env_var: "NONEXISTENT_VAR".to_string(),
            fallback: "fallback_value".to_string(),
        };

        let request = DefaultRequest {
            attribute_path: "test.field".to_string(),
        };

        let response = default.default_value(request);

        match response.value {
            Dynamic::String(val) => assert_eq!(val, "fallback_value"),
            _ => panic!("Expected String variant"),
        }
    }
}
