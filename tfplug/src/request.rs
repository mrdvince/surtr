use crate::context::Context;
use crate::types::{Config, Diagnostics, State};

#[derive(Clone)]
pub struct ConfigureRequest {
    pub context: Context,
    pub config: Config,
}

#[derive(Clone)]
pub struct ConfigureResponse {
    pub diagnostics: Diagnostics,
}

#[derive(Clone)]
pub struct SchemaRequest {
    pub context: Context,
}

pub struct ResourceSchemaResponse {
    pub schema: crate::provider::ResourceSchema,
    pub diagnostics: Diagnostics,
}

pub struct DataSourceSchemaResponse {
    pub schema: crate::provider::DataSourceSchema,
    pub diagnostics: Diagnostics,
}

#[derive(Clone)]
pub struct CreateRequest {
    pub context: Context,
    pub config: Config,
    pub planned_state: State,
}

#[derive(Clone)]
pub struct CreateResponse {
    pub state: State,
    pub diagnostics: Diagnostics,
}

#[derive(Clone)]
pub struct ReadRequest {
    pub context: Context,
    pub current_state: State,
}

#[derive(Clone)]
pub struct ReadResponse {
    pub state: Option<State>,
    pub diagnostics: Diagnostics,
}

#[derive(Clone)]
pub struct UpdateRequest {
    pub context: Context,
    pub config: Config,
    pub planned_state: State,
    pub current_state: State,
}

#[derive(Clone)]
pub struct UpdateResponse {
    pub state: State,
    pub diagnostics: Diagnostics,
}

#[derive(Clone)]
pub struct DeleteRequest {
    pub context: Context,
    pub current_state: State,
}

#[derive(Clone)]
pub struct DeleteResponse {
    pub diagnostics: Diagnostics,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Config, Dynamic, State};
    use std::collections::HashMap;

    #[test]
    fn configure_request_contains_config_and_context() {
        let ctx = Context::new();
        let config = Config {
            values: HashMap::new(),
        };

        let req = ConfigureRequest {
            context: ctx.clone(),
            config: config.clone(),
        };

        assert_eq!(req.config.values.len(), 0);
    }

    #[test]
    fn schema_request_contains_only_context() {
        let ctx = Context::new();
        let req = SchemaRequest { context: ctx };

        assert!(matches!(req.context, Context { .. }));
    }

    #[test]
    fn create_request_contains_config_and_planned_state() {
        let ctx = Context::new();
        let config = Config {
            values: HashMap::new(),
        };
        let planned_state = State {
            values: HashMap::new(),
        };

        let req = CreateRequest {
            context: ctx,
            config,
            planned_state,
        };

        assert_eq!(req.config.values.len(), 0);
        assert_eq!(req.planned_state.values.len(), 0);
    }

    #[test]
    fn read_request_contains_current_state() {
        let ctx = Context::new();
        let mut values = HashMap::new();
        values.insert("id".to_string(), Dynamic::String("test-123".to_string()));
        let current_state = State { values };

        let req = ReadRequest {
            context: ctx,
            current_state: current_state.clone(),
        };

        assert_eq!(req.current_state.values.len(), 1);
        assert_eq!(
            req.current_state
                .values
                .get("id")
                .and_then(|v| v.as_string()),
            Some(&"test-123".to_string())
        );
    }
}
