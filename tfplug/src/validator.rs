use crate::types::{Diagnostics, Dynamic};

pub trait Validator: Send + Sync {
    fn validate(&self, value: &Dynamic, attribute_path: &str, diagnostics: &mut Diagnostics);
}

pub struct StringLengthValidator {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl Validator for StringLengthValidator {
    fn validate(&self, value: &Dynamic, attribute_path: &str, diagnostics: &mut Diagnostics) {
        if let Some(s) = value.as_string() {
            if let Some(min) = self.min {
                if s.len() < min {
                    diagnostics.add_error(
                        format!("{} must have minimum length of {}", attribute_path, min),
                        Some(format!("Got length {}", s.len())),
                    );
                }
            }
            if let Some(max) = self.max {
                if s.len() > max {
                    diagnostics.add_error(
                        format!("{} must have maximum length of {}", attribute_path, max),
                        Some(format!("Got length {}", s.len())),
                    );
                }
            }
        }
    }
}

pub struct StringPatternValidator {
    pub pattern: regex::Regex,
    pub description: String,
}

impl Validator for StringPatternValidator {
    fn validate(&self, value: &Dynamic, attribute_path: &str, diagnostics: &mut Diagnostics) {
        if let Some(s) = value.as_string() {
            if !self.pattern.is_match(s) {
                diagnostics.add_error(
                    format!("{} must match {}", attribute_path, self.description),
                    Some(format!("Value '{}' does not match pattern", s)),
                );
            }
        }
    }
}

pub struct NumberRangeValidator {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Validator for NumberRangeValidator {
    fn validate(&self, value: &Dynamic, attribute_path: &str, diagnostics: &mut Diagnostics) {
        if let Some(n) = value.as_number() {
            if let Some(min) = self.min {
                if n < min {
                    diagnostics.add_error(
                        format!("{} must be at least {}", attribute_path, min),
                        Some(format!("Got {}", n)),
                    );
                }
            }
            if let Some(max) = self.max {
                if n > max {
                    diagnostics.add_error(
                        format!("{} must be at most {}", attribute_path, max),
                        Some(format!("Got {}", n)),
                    );
                }
            }
        }
    }
}

pub struct ListLengthValidator {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl Validator for ListLengthValidator {
    fn validate(&self, value: &Dynamic, attribute_path: &str, diagnostics: &mut Diagnostics) {
        if let Dynamic::List(items) = value {
            if let Some(min) = self.min {
                if items.len() < min {
                    diagnostics.add_error(
                        format!("{} must have at least {} items", attribute_path, min),
                        Some(format!("Got {} items", items.len())),
                    );
                }
            }
            if let Some(max) = self.max {
                if items.len() > max {
                    diagnostics.add_error(
                        format!("{} must have at most {} items", attribute_path, max),
                        Some(format!("Got {} items", items.len())),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostics, Dynamic};

    #[test]
    fn string_length_validator_accepts_valid_length() {
        let validator = StringLengthValidator {
            min: Some(3),
            max: Some(10),
        };

        let mut diags = Diagnostics::new();
        validator.validate(
            &Dynamic::String("hello".to_string()),
            "test_field",
            &mut diags,
        );

        assert_eq!(diags.errors.len(), 0);
    }

    #[test]
    fn string_length_validator_rejects_too_short() {
        let validator = StringLengthValidator {
            min: Some(5),
            max: None,
        };

        let mut diags = Diagnostics::new();
        validator.validate(&Dynamic::String("hi".to_string()), "test_field", &mut diags);

        assert_eq!(diags.errors.len(), 1);
        assert!(diags.errors[0].summary.contains("minimum length"));
    }

    #[test]
    fn string_length_validator_rejects_too_long() {
        let validator = StringLengthValidator {
            min: None,
            max: Some(5),
        };

        let mut diags = Diagnostics::new();
        validator.validate(
            &Dynamic::String("hello world".to_string()),
            "test_field",
            &mut diags,
        );

        assert_eq!(diags.errors.len(), 1);
        assert!(diags.errors[0].summary.contains("maximum length"));
    }

    #[test]
    fn string_pattern_validator_accepts_matching_pattern() {
        let validator = StringPatternValidator {
            pattern: regex::Regex::new(r"^\d{3}-\d{3}-\d{4}$").unwrap(),
            description: "phone number format".to_string(),
        };

        let mut diags = Diagnostics::new();
        validator.validate(
            &Dynamic::String("123-456-7890".to_string()),
            "phone",
            &mut diags,
        );

        assert_eq!(diags.errors.len(), 0);
    }

    #[test]
    fn string_pattern_validator_rejects_non_matching() {
        let validator = StringPatternValidator {
            pattern: regex::Regex::new(r"^\d{3}-\d{3}-\d{4}$").unwrap(),
            description: "phone number format".to_string(),
        };

        let mut diags = Diagnostics::new();
        validator.validate(&Dynamic::String("invalid".to_string()), "phone", &mut diags);

        assert_eq!(diags.errors.len(), 1);
        assert!(diags.errors[0].summary.contains("phone number format"));
    }

    #[test]
    fn number_range_validator_accepts_valid_number() {
        let validator = NumberRangeValidator {
            min: Some(1.0),
            max: Some(100.0),
        };

        let mut diags = Diagnostics::new();
        validator.validate(&Dynamic::Number(50.0), "count", &mut diags);

        assert_eq!(diags.errors.len(), 0);
    }

    #[test]
    fn number_range_validator_rejects_too_small() {
        let validator = NumberRangeValidator {
            min: Some(10.0),
            max: None,
        };

        let mut diags = Diagnostics::new();
        validator.validate(&Dynamic::Number(5.0), "count", &mut diags);

        assert_eq!(diags.errors.len(), 1);
        assert!(diags.errors[0].summary.contains("at least"));
    }

    #[test]
    fn list_length_validator_accepts_valid_length() {
        let validator = ListLengthValidator {
            min: Some(1),
            max: Some(5),
        };

        let mut diags = Diagnostics::new();
        let list = Dynamic::List(vec![
            Dynamic::String("a".to_string()),
            Dynamic::String("b".to_string()),
        ]);
        validator.validate(&list, "items", &mut diags);

        assert_eq!(diags.errors.len(), 0);
    }

    #[test]
    fn custom_validator_runs_custom_logic() {
        struct EvenNumberValidator;

        impl Validator for EvenNumberValidator {
            fn validate(
                &self,
                value: &Dynamic,
                attribute_path: &str,
                diagnostics: &mut Diagnostics,
            ) {
                if let Some(num) = value.as_number() {
                    if num as i64 % 2 != 0 {
                        diagnostics.add_error(
                            format!("{} must be an even number", attribute_path),
                            Some(format!("Got {}, which is odd", num)),
                        );
                    }
                }
            }
        }

        let validator = EvenNumberValidator;
        let mut diags = Diagnostics::new();

        validator.validate(&Dynamic::Number(4.0), "even_field", &mut diags);
        assert_eq!(diags.errors.len(), 0);

        validator.validate(&Dynamic::Number(3.0), "even_field", &mut diags);
        assert_eq!(diags.errors.len(), 1);
    }
}
