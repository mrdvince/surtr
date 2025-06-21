use crate::api::{Client, RealmConfig};
use std::collections::HashMap;
use tfplug::provider::ResourceSchema;
use tfplug::{AttributeBuilder, Config, Diagnostics, Dynamic, Resource, SchemaBuilder, State};

pub struct RealmResource {
    client: Client,
}

impl RealmResource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn schema_static() -> ResourceSchema {
        SchemaBuilder::new()
            .attribute(
                "realm",
                AttributeBuilder::string("realm")
                    .required()
                    .description("Realm/domain name"),
            )
            .attribute(
                "type",
                AttributeBuilder::string("type")
                    .required()
                    .description("Authentication type (e.g., 'openid')"),
            )
            .attribute(
                "issuer_url",
                AttributeBuilder::string("issuer_url")
                    .required()
                    .description("OpenID issuer URL"),
            )
            .attribute(
                "client_id",
                AttributeBuilder::string("client_id")
                    .required()
                    .description("OAuth/OpenID client ID"),
            )
            .attribute(
                "client_key",
                AttributeBuilder::string("client_key")
                    .required()
                    .sensitive()
                    .description("OAuth/OpenID client secret"),
            )
            .attribute(
                "username_claim",
                AttributeBuilder::string("username_claim")
                    .optional()
                    .description("OpenID claim used as username"),
            )
            .attribute(
                "autocreate",
                AttributeBuilder::bool("autocreate")
                    .optional()
                    .description("Automatically create users if they don't exist"),
            )
            .attribute(
                "default",
                AttributeBuilder::bool("default")
                    .optional()
                    .description("Set as default authentication realm"),
            )
            .attribute(
                "comment",
                AttributeBuilder::string("comment")
                    .optional()
                    .description("Description/comment for the realm"),
            )
            .build_resource(0)
    }
}

impl Resource for RealmResource {
    fn schema(&self) -> ResourceSchema {
        Self::schema_static()
    }

    fn create(&self, config: Config) -> tfplug::Result<(State, Diagnostics)> {
        let diags = Diagnostics::new();

        // Extract realm configuration from config
        let realm = config
            .values
            .get("realm")
            .and_then(|v| v.as_string())
            .ok_or("realm is required")?;

        let realm_type = config
            .values
            .get("type")
            .and_then(|v| v.as_string())
            .ok_or("type is required")?;

        let issuer_url = config
            .values
            .get("issuer_url")
            .and_then(|v| v.as_string())
            .ok_or("issuer_url is required")?;

        let client_id = config
            .values
            .get("client_id")
            .and_then(|v| v.as_string())
            .ok_or("client_id is required")?;

        let client_key = config
            .values
            .get("client_key")
            .and_then(|v| v.as_string())
            .ok_or("client_key is required")?;

        let realm_config = RealmConfig {
            realm: realm.to_string(),
            realm_type: realm_type.to_string(),
            issuer_url: issuer_url.to_string(),
            client_id: client_id.to_string(),
            client_key: client_key.to_string(),
            username_claim: config
                .values
                .get("username_claim")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
            autocreate: config.values.get("autocreate").and_then(|v| v.as_bool()),
            default: config.values.get("default").and_then(|v| v.as_bool()),
            comment: config
                .values
                .get("comment")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
        };

        // Create the realm using async API
        let client = self.client.clone();
        let create_result = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
                .block_on(async move { client.create_realm(realm_config).await })
        });

        tracing::debug!("Create realm result: {:?}", create_result);
        create_result.map_err(|e| format!("Failed to create realm: {}", e))?;

        // Build the state with all required attributes
        let mut state_values = HashMap::new();

        // Add all required attributes to state
        state_values.insert("realm".to_string(), Dynamic::String(realm.to_string()));
        state_values.insert("type".to_string(), Dynamic::String(realm_type.to_string()));
        state_values.insert(
            "issuer_url".to_string(),
            Dynamic::String(issuer_url.to_string()),
        );
        state_values.insert(
            "client_id".to_string(),
            Dynamic::String(client_id.to_string()),
        );
        state_values.insert(
            "client_key".to_string(),
            Dynamic::String(client_key.to_string()),
        );

        // Add optional attributes if present
        if let Some(username_claim) = config
            .values
            .get("username_claim")
            .and_then(|v| v.as_string())
        {
            state_values.insert(
                "username_claim".to_string(),
                Dynamic::String(username_claim.to_string()),
            );
        }
        if let Some(autocreate) = config.values.get("autocreate").and_then(|v| v.as_bool()) {
            state_values.insert("autocreate".to_string(), Dynamic::Bool(autocreate));
        }
        if let Some(default) = config.values.get("default").and_then(|v| v.as_bool()) {
            state_values.insert("default".to_string(), Dynamic::Bool(default));
        }
        if let Some(comment) = config.values.get("comment").and_then(|v| v.as_string()) {
            state_values.insert("comment".to_string(), Dynamic::String(comment.to_string()));
        }

        // Return the created state
        Ok((
            State {
                values: state_values,
            },
            diags,
        ))
    }

    fn read(&self, state: State) -> tfplug::Result<(Option<State>, Diagnostics)> {
        let diags = Diagnostics::new();

        let realm = state
            .values
            .get("realm")
            .and_then(|v| v.as_string())
            .ok_or("realm is required in state")?;

        // Read realm info using async API
        let client = self.client.clone();
        let realm_info = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move { client.get_realm(realm).await })
        })
        .map_err(|e| format!("Failed to read realm: {}", e))?;

        match realm_info {
            Some(info) => {
                // Build new state from API response
                let mut values = HashMap::new();
                // Realm name comes from state, not API response
                values.insert("realm".to_string(), Dynamic::String(realm.to_string()));
                values.insert("type".to_string(), Dynamic::String(info.realm_type));
                values.insert("issuer_url".to_string(), Dynamic::String(info.issuer_url));
                values.insert("client_id".to_string(), Dynamic::String(info.client_id));

                // Preserve sensitive client_key from state
                if let Some(client_key) = state.values.get("client_key") {
                    values.insert("client_key".to_string(), client_key.clone());
                }

                if let Some(username_claim) = info.username_claim {
                    values.insert(
                        "username_claim".to_string(),
                        Dynamic::String(username_claim),
                    );
                }

                if let Some(autocreate) = info.autocreate {
                    values.insert("autocreate".to_string(), Dynamic::Bool(autocreate != 0));
                }

                if let Some(default) = info.default {
                    values.insert("default".to_string(), Dynamic::Bool(default != 0));
                }

                if let Some(comment) = info.comment {
                    values.insert("comment".to_string(), Dynamic::String(comment));
                }

                Ok((Some(State { values }), diags))
            }
            None => Ok((None, diags)), // Resource doesn't exist
        }
    }

    fn update(&self, _state: State, config: Config) -> tfplug::Result<(State, Diagnostics)> {
        let diags = Diagnostics::new();

        let realm = config
            .values
            .get("realm")
            .and_then(|v| v.as_string())
            .ok_or("realm is required")?;

        let realm_type = config
            .values
            .get("type")
            .and_then(|v| v.as_string())
            .ok_or("type is required")?;

        let issuer_url = config
            .values
            .get("issuer_url")
            .and_then(|v| v.as_string())
            .ok_or("issuer_url is required")?;

        let client_id = config
            .values
            .get("client_id")
            .and_then(|v| v.as_string())
            .ok_or("client_id is required")?;

        let client_key = config
            .values
            .get("client_key")
            .and_then(|v| v.as_string())
            .ok_or("client_key is required")?;

        let realm_config = RealmConfig {
            realm: realm.to_string(),
            realm_type: realm_type.to_string(),
            issuer_url: issuer_url.to_string(),
            client_id: client_id.to_string(),
            client_key: client_key.to_string(),
            username_claim: config
                .values
                .get("username_claim")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
            autocreate: config.values.get("autocreate").and_then(|v| v.as_bool()),
            default: config.values.get("default").and_then(|v| v.as_bool()),
            comment: config
                .values
                .get("comment")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
        };

        // Update the realm using async API
        let client = self.client.clone();
        let realm_name = realm.to_string();
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
                .block_on(async move { client.update_realm(&realm_name, realm_config).await })
        })
        .map_err(|e| format!("Failed to update realm: {}", e))?;

        // Return the updated state
        Ok((
            State {
                values: config.values,
            },
            diags,
        ))
    }

    fn delete(&self, state: State) -> tfplug::Result<Diagnostics> {
        let diags = Diagnostics::new();

        let realm = state
            .values
            .get("realm")
            .and_then(|v| v.as_string())
            .ok_or("realm is required in state")?;

        // Delete the realm using async API
        let client = self.client.clone();
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
                .block_on(async move { client.delete_realm(realm).await })
        })
        .map_err(|e| format!("Failed to delete realm: {}", e))?;

        Ok(diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Client;
    use mockito::Server;

    fn create_test_client(server_url: String) -> Client {
        Client::new(server_url, "test@pve!token=secret".to_string(), true).unwrap()
    }

    fn create_test_config() -> Config {
        let mut values = HashMap::new();
        values.insert(
            "realm".to_string(),
            Dynamic::String("test-realm".to_string()),
        );
        values.insert("type".to_string(), Dynamic::String("openid".to_string()));
        values.insert(
            "issuer_url".to_string(),
            Dynamic::String("https://auth.example.com".to_string()),
        );
        values.insert(
            "client_id".to_string(),
            Dynamic::String("test-client".to_string()),
        );
        values.insert(
            "client_key".to_string(),
            Dynamic::String("test-secret".to_string()),
        );
        Config { values }
    }

    #[test]
    fn resource_has_correct_schema() {
        let schema = RealmResource::schema_static();

        // Check required attributes
        assert!(schema.attributes.contains_key("realm"));
        assert!(schema.attributes["realm"].required);

        assert!(schema.attributes.contains_key("type"));
        assert!(schema.attributes["type"].required);

        assert!(schema.attributes.contains_key("issuer_url"));
        assert!(schema.attributes["issuer_url"].required);

        assert!(schema.attributes.contains_key("client_id"));
        assert!(schema.attributes["client_id"].required);

        assert!(schema.attributes.contains_key("client_key"));
        assert!(schema.attributes["client_key"].required);
        assert!(schema.attributes["client_key"].sensitive);

        // Check optional attributes
        assert!(schema.attributes.contains_key("username_claim"));
        assert!(schema.attributes["username_claim"].optional);

        assert!(schema.attributes.contains_key("autocreate"));
        assert!(schema.attributes["autocreate"].optional);

        assert!(schema.attributes.contains_key("default"));
        assert!(schema.attributes["default"].optional);

        assert!(schema.attributes.contains_key("comment"));
        assert!(schema.attributes["comment"].optional);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resource_creates_realm_successfully() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/api2/json/access/domains")
            .with_status(200)
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let resource = RealmResource::new(client);
        let config = create_test_config();

        let result = resource.create(config);
        assert!(result.is_ok());

        let (state, diags) = result.unwrap();
        assert!(diags.errors.is_empty());
        match state.values.get("realm").unwrap() {
            Dynamic::String(s) => assert_eq!(s, "test-realm"),
            _ => panic!("Expected realm to be a string"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resource_handles_creation_failure() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/api2/json/access/domains")
            .with_status(400)
            .with_body(r#"{"errors":{"realm":"domain 'test-realm' already exists"}}"#)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let resource = RealmResource::new(client);
        let config = create_test_config();

        let result = resource.create(config);
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resource_reads_existing_realm() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api2/json/access/domains/test-realm")
            .with_status(200)
            .with_body(
                r#"{
                "data": {
                    "realm": "test-realm",
                    "type": "openid",
                    "issuer-url": "https://auth.example.com",
                    "client-id": "test-client",
                    "username-claim": "email",
                    "autocreate": 1,
                    "default": 0
                }
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let resource = RealmResource::new(client);

        let mut state_values = HashMap::new();
        state_values.insert(
            "realm".to_string(),
            Dynamic::String("test-realm".to_string()),
        );
        let state = State {
            values: state_values,
        };

        let result = resource.read(state);
        assert!(result.is_ok());

        let (new_state, diags) = result.unwrap();
        assert!(diags.errors.is_empty());
        assert!(new_state.is_some());

        let state = new_state.unwrap();
        match state.values.get("username_claim").unwrap() {
            Dynamic::String(s) => assert_eq!(s, "email"),
            _ => panic!("Expected username_claim to be a string"),
        }
        match state.values.get("autocreate").unwrap() {
            Dynamic::Bool(b) => assert_eq!(*b, true),
            _ => panic!("Expected autocreate to be a bool"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resource_handles_read_not_found() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api2/json/access/domains/test-realm")
            .with_status(500)
            .with_body(r#"{"errors":{"realm":"domain 'test-realm' does not exist"}}"#)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let resource = RealmResource::new(client);

        let mut state_values = HashMap::new();
        state_values.insert(
            "realm".to_string(),
            Dynamic::String("test-realm".to_string()),
        );
        let state = State {
            values: state_values,
        };

        let result = resource.read(state);
        assert!(result.is_ok());

        let (new_state, diags) = result.unwrap();
        assert!(diags.errors.is_empty());
        assert!(new_state.is_none()); // Resource doesn't exist
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resource_updates_realm() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("PUT", "/api2/json/access/domains/test-realm")
            .with_status(200)
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let resource = RealmResource::new(client);

        let mut state_values = HashMap::new();
        state_values.insert(
            "realm".to_string(),
            Dynamic::String("test-realm".to_string()),
        );
        state_values.insert("type".to_string(), Dynamic::String("openid".to_string()));
        let state = State {
            values: state_values,
        };

        let mut config = create_test_config();
        config.values.insert(
            "issuer_url".to_string(),
            Dynamic::String("https://new-auth.example.com".to_string()),
        );

        let result = resource.update(state, config);
        assert!(result.is_ok());

        let (new_state, diags) = result.unwrap();
        assert!(diags.errors.is_empty());
        match new_state.values.get("issuer_url").unwrap() {
            Dynamic::String(s) => assert_eq!(s, "https://new-auth.example.com"),
            _ => panic!("Expected issuer_url to be a string"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resource_deletes_realm() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("DELETE", "/api2/json/access/domains/test-realm")
            .with_status(200)
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let resource = RealmResource::new(client);

        let mut state_values = HashMap::new();
        state_values.insert(
            "realm".to_string(),
            Dynamic::String("test-realm".to_string()),
        );
        let state = State {
            values: state_values,
        };

        let result = resource.delete(state);
        assert!(result.is_ok());

        let diags = result.unwrap();
        assert!(diags.errors.is_empty());
    }
}
