use crate::api::{Client, RealmConfig};
use async_trait::async_trait;
use std::collections::HashMap;
use tfplug::request::{
    CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, ReadRequest, ReadResponse,
    ResourceSchemaResponse, SchemaRequest, UpdateRequest, UpdateResponse,
};
use tfplug::types::{Diagnostics, Dynamic, State};
use tfplug::ResourceV2;
use tfplug::{AttributeBuilder, SchemaBuilder};

pub struct RealmResource {
    client: Client,
}

impl RealmResource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn schema_static() -> tfplug::provider::ResourceSchema {
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
            .attribute(
                "groups_overwrite",
                AttributeBuilder::bool("groups_overwrite")
                    .optional()
                    .description("Overwrite group membership from authentication provider"),
            )
            .attribute(
                "groups_autocreate",
                AttributeBuilder::bool("groups_autocreate")
                    .optional()
                    .description("Automatically create groups if they don't exist"),
            )
            .build_resource(1)
    }
}

#[async_trait]
impl ResourceV2 for RealmResource {
    async fn schema(&self, _request: SchemaRequest) -> ResourceSchemaResponse {
        let schema = SchemaBuilder::new()
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
            .attribute(
                "groups_overwrite",
                AttributeBuilder::bool("groups_overwrite")
                    .optional()
                    .description("Overwrite group membership from authentication provider"),
            )
            .attribute(
                "groups_autocreate",
                AttributeBuilder::bool("groups_autocreate")
                    .optional()
                    .description("Automatically create groups if they don't exist"),
            )
            .build_resource(0);

        ResourceSchemaResponse {
            schema,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn create(&self, request: CreateRequest) -> CreateResponse {
        let mut diags = Diagnostics::new();

        let realm = match request
            .config
            .values
            .get("realm")
            .and_then(|v| v.as_string())
        {
            Some(realm) => realm,
            None => {
                diags.add_error("realm is required", None::<String>);
                return CreateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let realm_type = match request
            .config
            .values
            .get("type")
            .and_then(|v| v.as_string())
        {
            Some(realm_type) => realm_type,
            None => {
                diags.add_error("type is required", None::<String>);
                return CreateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let issuer_url = match request
            .config
            .values
            .get("issuer_url")
            .and_then(|v| v.as_string())
        {
            Some(issuer_url) => issuer_url,
            None => {
                diags.add_error("issuer_url is required", None::<String>);
                return CreateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let client_id = match request
            .config
            .values
            .get("client_id")
            .and_then(|v| v.as_string())
        {
            Some(client_id) => client_id,
            None => {
                diags.add_error("client_id is required", None::<String>);
                return CreateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let client_key = match request
            .config
            .values
            .get("client_key")
            .and_then(|v| v.as_string())
        {
            Some(client_key) => client_key,
            None => {
                diags.add_error("client_key is required", None::<String>);
                return CreateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let realm_config = RealmConfig {
            realm: realm.to_string(),
            realm_type: realm_type.to_string(),
            issuer_url: issuer_url.to_string(),
            client_id: client_id.to_string(),
            client_key: client_key.to_string(),
            username_claim: request
                .config
                .values
                .get("username_claim")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
            autocreate: request
                .config
                .values
                .get("autocreate")
                .and_then(|v| v.as_bool()),
            default: request
                .config
                .values
                .get("default")
                .and_then(|v| v.as_bool()),
            groups_overwrite: request
                .config
                .values
                .get("groups_overwrite")
                .and_then(|v| v.as_bool()),
            groups_autocreate: request
                .config
                .values
                .get("groups_autocreate")
                .and_then(|v| v.as_bool()),
            comment: request
                .config
                .values
                .get("comment")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
        };

        let create_result = self.client.create_realm(realm_config).await;

        match create_result {
            Ok(_) => {
                let mut state_values = HashMap::new();

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

                if let Some(username_claim) = request.config.values.get("username_claim") {
                    state_values.insert("username_claim".to_string(), username_claim.clone());
                }

                if let Some(autocreate) = request.config.values.get("autocreate") {
                    state_values.insert("autocreate".to_string(), autocreate.clone());
                }

                if let Some(default) = request.config.values.get("default") {
                    state_values.insert("default".to_string(), default.clone());
                }

                if let Some(comment) = request.config.values.get("comment") {
                    state_values.insert("comment".to_string(), comment.clone());
                }

                if let Some(groups_overwrite) = request.config.values.get("groups_overwrite") {
                    state_values.insert("groups_overwrite".to_string(), groups_overwrite.clone());
                }

                if let Some(groups_autocreate) = request.config.values.get("groups_autocreate") {
                    state_values.insert("groups_autocreate".to_string(), groups_autocreate.clone());
                }

                CreateResponse {
                    state: State {
                        values: state_values,
                    },
                    diagnostics: diags,
                }
            }
            Err(e) => {
                diags.add_error(format!("Failed to create realm: {}", e), None::<String>);
                CreateResponse {
                    state: State::new(),
                    diagnostics: diags,
                }
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> ReadResponse {
        let mut diags = Diagnostics::new();

        let realm = match request
            .current_state
            .values
            .get("realm")
            .and_then(|v| v.as_string())
        {
            Some(realm) => realm,
            None => {
                diags.add_error("realm is required in state", None::<String>);
                return ReadResponse {
                    state: None,
                    diagnostics: diags,
                };
            }
        };

        let realm_result = self.client.get_realm(realm).await;

        match realm_result {
            Ok(Some(info)) => {
                let mut values = HashMap::new();
                values.insert("realm".to_string(), Dynamic::String(realm.to_string()));
                values.insert("type".to_string(), Dynamic::String(info.realm_type));
                values.insert("issuer_url".to_string(), Dynamic::String(info.issuer_url));
                values.insert("client_id".to_string(), Dynamic::String(info.client_id));

                if let Some(client_key) = request.current_state.values.get("client_key") {
                    values.insert("client_key".to_string(), client_key.clone());
                }

                match info.username_claim {
                    Some(username_claim) => {
                        values.insert(
                            "username_claim".to_string(),
                            Dynamic::String(username_claim),
                        );
                    }
                    None => {
                        if let Some(v) = request.current_state.values.get("username_claim") {
                            values.insert("username_claim".to_string(), v.clone());
                        }
                    }
                }

                match info.autocreate {
                    Some(autocreate) => {
                        values.insert("autocreate".to_string(), Dynamic::Bool(autocreate != 0));
                    }
                    None => {
                        if let Some(v) = request.current_state.values.get("autocreate") {
                            values.insert("autocreate".to_string(), v.clone());
                        }
                    }
                }

                match info.default {
                    Some(default) => {
                        values.insert("default".to_string(), Dynamic::Bool(default != 0));
                    }
                    None => {
                        if let Some(v) = request.current_state.values.get("default") {
                            values.insert("default".to_string(), v.clone());
                        }
                    }
                }

                match info.comment {
                    Some(comment) => {
                        values.insert("comment".to_string(), Dynamic::String(comment));
                    }
                    None => {
                        if let Some(v) = request.current_state.values.get("comment") {
                            values.insert("comment".to_string(), v.clone());
                        }
                    }
                }

                match info.groups_overwrite {
                    Some(groups_overwrite) => {
                        values.insert(
                            "groups_overwrite".to_string(),
                            Dynamic::Bool(groups_overwrite != 0),
                        );
                    }
                    None => {
                        if let Some(v) = request.current_state.values.get("groups_overwrite") {
                            values.insert("groups_overwrite".to_string(), v.clone());
                        }
                    }
                }

                match info.groups_autocreate {
                    Some(groups_autocreate) => {
                        values.insert(
                            "groups_autocreate".to_string(),
                            Dynamic::Bool(groups_autocreate != 0),
                        );
                    }
                    None => {
                        if let Some(v) = request.current_state.values.get("groups_autocreate") {
                            values.insert("groups_autocreate".to_string(), v.clone());
                        }
                    }
                }

                ReadResponse {
                    state: Some(State { values }),
                    diagnostics: diags,
                }
            }
            Ok(None) => ReadResponse {
                state: None,
                diagnostics: diags,
            },
            Err(e) => {
                tracing::debug!("Failed to read realm: {}", e);
                ReadResponse {
                    state: None,
                    diagnostics: diags,
                }
            }
        }
    }

    async fn update(&self, request: UpdateRequest) -> UpdateResponse {
        let mut diags = Diagnostics::new();

        let realm = match request
            .config
            .values
            .get("realm")
            .and_then(|v| v.as_string())
        {
            Some(realm) => realm,
            None => {
                diags.add_error("realm is required", None::<String>);
                return UpdateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let realm_type = match request
            .config
            .values
            .get("type")
            .and_then(|v| v.as_string())
        {
            Some(realm_type) => realm_type,
            None => {
                diags.add_error("type is required", None::<String>);
                return UpdateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let issuer_url = match request
            .config
            .values
            .get("issuer_url")
            .and_then(|v| v.as_string())
        {
            Some(issuer_url) => issuer_url,
            None => {
                diags.add_error("issuer_url is required", None::<String>);
                return UpdateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let client_id = match request
            .config
            .values
            .get("client_id")
            .and_then(|v| v.as_string())
        {
            Some(client_id) => client_id,
            None => {
                diags.add_error("client_id is required", None::<String>);
                return UpdateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let client_key = match request
            .config
            .values
            .get("client_key")
            .and_then(|v| v.as_string())
        {
            Some(client_key) => client_key,
            None => {
                diags.add_error("client_key is required", None::<String>);
                return UpdateResponse {
                    state: State::new(),
                    diagnostics: diags,
                };
            }
        };

        let realm_config = RealmConfig {
            realm: realm.to_string(),
            realm_type: realm_type.to_string(),
            issuer_url: issuer_url.to_string(),
            client_id: client_id.to_string(),
            client_key: client_key.to_string(),
            username_claim: request
                .config
                .values
                .get("username_claim")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
            autocreate: request
                .config
                .values
                .get("autocreate")
                .and_then(|v| v.as_bool()),
            default: request
                .config
                .values
                .get("default")
                .and_then(|v| v.as_bool()),
            groups_overwrite: request
                .config
                .values
                .get("groups_overwrite")
                .and_then(|v| v.as_bool()),
            groups_autocreate: request
                .config
                .values
                .get("groups_autocreate")
                .and_then(|v| v.as_bool()),
            comment: request
                .config
                .values
                .get("comment")
                .and_then(|v| v.as_string())
                .map(|s| s.to_string()),
        };

        let update_result = self
            .client
            .update_realm(&realm.to_string(), realm_config)
            .await;

        match update_result {
            Ok(_) => {
                let mut updated_values = HashMap::new();
                for (key, value) in request.config.values {
                    updated_values.insert(key, value);
                }

                UpdateResponse {
                    state: State {
                        values: updated_values,
                    },
                    diagnostics: diags,
                }
            }
            Err(e) => {
                diags.add_error(format!("Failed to update realm: {}", e), None::<String>);
                UpdateResponse {
                    state: request.current_state,
                    diagnostics: diags,
                }
            }
        }
    }

    async fn delete(&self, request: DeleteRequest) -> DeleteResponse {
        let mut diags = Diagnostics::new();

        let realm = match request
            .current_state
            .values
            .get("realm")
            .and_then(|v| v.as_string())
        {
            Some(realm) => realm,
            None => {
                diags.add_error("realm is required in state", None::<String>);
                return DeleteResponse { diagnostics: diags };
            }
        };

        let delete_result = self.client.delete_realm(realm).await;

        match delete_result {
            Ok(_) => DeleteResponse { diagnostics: diags },
            Err(e) => {
                diags.add_error(format!("Failed to delete realm: {}", e), None::<String>);
                DeleteResponse { diagnostics: diags }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::api::Client;
    use mockito::Server;
    use tfplug::context::Context;
    use tfplug::types::Config;

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

        let request = CreateRequest {
            context: Context::new(),
            config: create_test_config(),
            planned_state: State::new(),
        };

        let response = resource.create(request).await;
        assert!(response.diagnostics.errors.is_empty());
        match response.state.values.get("realm").unwrap() {
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

        let request = CreateRequest {
            context: Context::new(),
            config: create_test_config(),
            planned_state: State::new(),
        };

        let response = resource.create(request).await;
        assert!(!response.diagnostics.errors.is_empty());
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

        let request = ReadRequest {
            context: Context::new(),
            current_state: State {
                values: state_values,
            },
        };

        let response = resource.read(request).await;
        assert!(response.diagnostics.errors.is_empty());
        assert!(response.state.is_some());

        let state = response.state.unwrap();
        match state.values.get("username_claim").unwrap() {
            Dynamic::String(s) => assert_eq!(s, "email"),
            _ => panic!("Expected username_claim to be a string"),
        }
        match state.values.get("autocreate").unwrap() {
            Dynamic::Bool(b) => assert!(*b),
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

        let request = ReadRequest {
            context: Context::new(),
            current_state: State {
                values: state_values,
            },
        };

        let response = resource.read(request).await;
        assert!(response.diagnostics.errors.is_empty());
        assert!(response.state.is_none());
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

        let request = DeleteRequest {
            context: Context::new(),
            current_state: State {
                values: state_values,
            },
        };

        let response = resource.delete(request).await;
        assert!(response.diagnostics.errors.is_empty());
    }

    #[tokio::test]
    async fn resource_has_correct_schema() {
        let client = create_test_client("http://test".to_string());
        let resource = RealmResource::new(client);

        let response = resource
            .schema(SchemaRequest {
                context: Context::new(),
            })
            .await;

        assert!(response.schema.attributes.contains_key("realm"));
        assert!(response.schema.attributes["realm"].required);

        assert!(response.schema.attributes.contains_key("type"));
        assert!(response.schema.attributes["type"].required);

        assert!(response.schema.attributes.contains_key("issuer_url"));
        assert!(response.schema.attributes["issuer_url"].required);

        assert!(response.schema.attributes.contains_key("client_id"));
        assert!(response.schema.attributes["client_id"].required);

        assert!(response.schema.attributes.contains_key("client_key"));
        assert!(response.schema.attributes["client_key"].required);
        assert!(response.schema.attributes["client_key"].sensitive);

        assert!(response.schema.attributes.contains_key("username_claim"));
        assert!(response.schema.attributes["username_claim"].optional);

        assert!(response.schema.attributes.contains_key("autocreate"));
        assert!(response.schema.attributes["autocreate"].optional);

        assert!(response.schema.attributes.contains_key("default"));
        assert!(response.schema.attributes["default"].optional);

        assert!(response.schema.attributes.contains_key("comment"));
        assert!(response.schema.attributes["comment"].optional);
    }
}
