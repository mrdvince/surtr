//! Validators for attribute validation
//!
//! This module provides built-in validators and the trait for custom validators.

use crate::schema::{Validator, ValidatorRequest, ValidatorResponse};
use crate::types::{Diagnostic, Dynamic};

/// String length validator - validates string minimum and maximum length
pub struct StringLengthValidator {
    min: Option<usize>,
    max: Option<usize>,
}

impl StringLengthValidator {
    /// Create a validator with minimum length
    pub fn min(length: usize) -> Box<dyn Validator> {
        Box::new(Self {
            min: Some(length),
            max: None,
        })
    }

    /// Create a validator with maximum length
    pub fn max(length: usize) -> Box<dyn Validator> {
        Box::new(Self {
            min: None,
            max: Some(length),
        })
    }

    /// Create a validator with both min and max length
    pub fn between(min: usize, max: usize) -> Box<dyn Validator> {
        Box::new(Self {
            min: Some(min),
            max: Some(max),
        })
    }
}

impl Validator for StringLengthValidator {
    fn description(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("string length must be between {} and {}", min, max),
            (Some(min), None) => format!("string length must be at least {}", min),
            (None, Some(max)) => format!("string length must be at most {}", max),
            (None, None) => "string length validator".to_string(),
        }
    }

    fn validate(&self, request: ValidatorRequest) -> ValidatorResponse {
        let mut diagnostics = Vec::new();

        if let Dynamic::String(s) = &request.config_value.value {
            let len = s.len();

            if let Some(min) = self.min {
                if len < min {
                    diagnostics.push(
                        Diagnostic::error(
                            "String too short",
                            format!("String length {} is less than minimum {}", len, min),
                        )
                        .with_attribute(request.path.clone()),
                    );
                }
            }

            if let Some(max) = self.max {
                if len > max {
                    diagnostics.push(
                        Diagnostic::error(
                            "String too long",
                            format!("String length {} is greater than maximum {}", len, max),
                        )
                        .with_attribute(request.path.clone()),
                    );
                }
            }
        }

        ValidatorResponse { diagnostics }
    }
}

/// Validates that a string matches one of the allowed values
pub struct StringOneOfValidator {
    allowed: Vec<String>,
}

impl StringOneOfValidator {
    /// Create a validator that ensures the value is one of the allowed strings
    pub fn create(allowed: Vec<String>) -> Box<dyn Validator> {
        Box::new(Self { allowed })
    }
}

impl Validator for StringOneOfValidator {
    fn description(&self) -> String {
        format!("value must be one of: {:?}", self.allowed)
    }

    fn validate(&self, request: ValidatorRequest) -> ValidatorResponse {
        let mut diagnostics = Vec::new();

        if let Dynamic::String(s) = &request.config_value.value {
            if !self.allowed.contains(s) {
                diagnostics.push(
                    Diagnostic::error(
                        "Invalid value",
                        format!(
                            "\"{}\" is not one of the allowed values: {:?}",
                            s, self.allowed
                        ),
                    )
                    .with_attribute(request.path),
                );
            }
        }

        ValidatorResponse { diagnostics }
    }
}

/// Validates that a number is within a range
pub struct NumberRangeValidator {
    min: Option<f64>,
    max: Option<f64>,
}

impl NumberRangeValidator {
    /// Create a validator with minimum value
    pub fn min(value: f64) -> Box<dyn Validator> {
        Box::new(Self {
            min: Some(value),
            max: None,
        })
    }

    /// Create a validator with maximum value
    pub fn max(value: f64) -> Box<dyn Validator> {
        Box::new(Self {
            min: None,
            max: Some(value),
        })
    }

    /// Create a validator with both min and max value
    pub fn between(min: f64, max: f64) -> Box<dyn Validator> {
        Box::new(Self {
            min: Some(min),
            max: Some(max),
        })
    }
}

impl Validator for NumberRangeValidator {
    fn description(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("number must be between {} and {}", min, max),
            (Some(min), None) => format!("number must be at least {}", min),
            (None, Some(max)) => format!("number must be at most {}", max),
            (None, None) => "number range validator".to_string(),
        }
    }

    fn validate(&self, request: ValidatorRequest) -> ValidatorResponse {
        let mut diagnostics = Vec::new();

        if let Dynamic::Number(n) = &request.config_value.value {
            if let Some(min) = self.min {
                if n < &min {
                    diagnostics.push(
                        Diagnostic::error(
                            "Number too small",
                            format!("Value {} is less than minimum {}", n, min),
                        )
                        .with_attribute(request.path.clone()),
                    );
                }
            }

            if let Some(max) = self.max {
                if n > &max {
                    diagnostics.push(
                        Diagnostic::error(
                            "Number too large",
                            format!("Value {} is greater than maximum {}", n, max),
                        )
                        .with_attribute(request.path.clone()),
                    );
                }
            }
        }

        ValidatorResponse { diagnostics }
    }
}

/// Validates list length
pub struct ListLengthValidator {
    min: Option<usize>,
    max: Option<usize>,
}

impl ListLengthValidator {
    /// Create a validator with minimum length
    pub fn min(length: usize) -> Box<dyn Validator> {
        Box::new(Self {
            min: Some(length),
            max: None,
        })
    }

    /// Create a validator with maximum length
    pub fn max(length: usize) -> Box<dyn Validator> {
        Box::new(Self {
            min: None,
            max: Some(length),
        })
    }

    /// Create a validator with both min and max length
    pub fn between(min: usize, max: usize) -> Box<dyn Validator> {
        Box::new(Self {
            min: Some(min),
            max: Some(max),
        })
    }
}

impl Validator for ListLengthValidator {
    fn description(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("list must have between {} and {} items", min, max),
            (Some(min), None) => format!("list must have at least {} items", min),
            (None, Some(max)) => format!("list must have at most {} items", max),
            (None, None) => "list length validator".to_string(),
        }
    }

    fn validate(&self, request: ValidatorRequest) -> ValidatorResponse {
        let mut diagnostics = Vec::new();

        if let Dynamic::List(l) = &request.config_value.value {
            let len = l.len();

            if let Some(min) = self.min {
                if len < min {
                    diagnostics.push(
                        Diagnostic::error(
                            "List too short",
                            format!("List has {} items, minimum is {}", len, min),
                        )
                        .with_attribute(request.path.clone()),
                    );
                }
            }

            if let Some(max) = self.max {
                if len > max {
                    diagnostics.push(
                        Diagnostic::error(
                            "List too long",
                            format!("List has {} items, maximum is {}", len, max),
                        )
                        .with_attribute(request.path.clone()),
                    );
                }
            }
        }

        ValidatorResponse { diagnostics }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AttributePath, DynamicValue};

    #[test]
    fn string_length_validator_validates_min() {
        let validator = StringLengthValidator::min(3);

        let request = ValidatorRequest {
            config_value: DynamicValue::new(Dynamic::String("ab".to_string())),
            path: AttributePath::new("test"),
        };

        let response = validator.validate(request);
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0].summary.contains("too short"));
    }

    #[test]
    fn string_one_of_validator() {
        let validator = StringOneOfValidator::create(vec!["foo".to_string(), "bar".to_string()]);

        let request = ValidatorRequest {
            config_value: DynamicValue::new(Dynamic::String("baz".to_string())),
            path: AttributePath::new("test"),
        };

        let response = validator.validate(request);
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0]
            .detail
            .contains("not one of the allowed values"));
    }

    #[test]
    fn number_range_validator() {
        let validator = NumberRangeValidator::between(1.0, 10.0);

        let request = ValidatorRequest {
            config_value: DynamicValue::new(Dynamic::Number(15.0)),
            path: AttributePath::new("test"),
        };

        let response = validator.validate(request);
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0].summary.contains("too large"));
    }
}
