use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub values: HashMap<String, Dynamic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub values: HashMap<String, Dynamic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dynamic {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<Dynamic>),
    Map(HashMap<String, Dynamic>),
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug)]
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

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}
