use crate::api::Client;
use async_trait::async_trait;
use std::collections::HashMap;
use tfplug::request::{DataSourceSchemaResponse, ReadRequest, ReadResponse, SchemaRequest};
use tfplug::types::{Diagnostics, Dynamic, State};
use tfplug::DataSourceV2;
use tfplug::{AttributeBuilder, SchemaBuilder};

pub struct VersionDataSource {
    client: Client,
}

impl VersionDataSource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn schema_static() -> tfplug::provider::DataSourceSchema {
        SchemaBuilder::new()
            .attribute("id", AttributeBuilder::string("id").computed())
            .attribute(
                "version",
                AttributeBuilder::string("version")
                    .computed()
                    .description("Proxmox version"),
            )
            .attribute(
                "repoid",
                AttributeBuilder::string("repoid")
                    .computed()
                    .description("Repository ID"),
            )
            .attribute(
                "release",
                AttributeBuilder::string("release")
                    .computed()
                    .description("Release version"),
            )
            .build_data_source(1)
    }
}

#[async_trait]
impl DataSourceV2 for VersionDataSource {
    async fn schema(&self, _request: SchemaRequest) -> DataSourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .attribute("id", AttributeBuilder::string("id").computed())
            .attribute(
                "version",
                AttributeBuilder::string("version")
                    .computed()
                    .description("Proxmox version"),
            )
            .attribute(
                "release",
                AttributeBuilder::string("release")
                    .computed()
                    .description("Proxmox release"),
            )
            .attribute(
                "repoid",
                AttributeBuilder::string("repoid")
                    .computed()
                    .description("Repository ID"),
            )
            .build_data_source(0);

        DataSourceSchemaResponse {
            schema,
            diagnostics: Diagnostics::new(),
        }
    }

    async fn read(&self, _request: ReadRequest) -> ReadResponse {
        let mut diags = Diagnostics::new();

        let version_result = self.client.get_version().await;

        match version_result {
            Ok(version_info) => {
                let mut values = HashMap::new();
                values.insert(
                    "id".to_string(),
                    Dynamic::String("proxmox_version".to_string()),
                );
                values.insert("version".to_string(), Dynamic::String(version_info.version));
                values.insert("release".to_string(), Dynamic::String(version_info.release));
                values.insert("repoid".to_string(), Dynamic::String(version_info.repoid));

                ReadResponse {
                    state: Some(State { values }),
                    diagnostics: diags,
                }
            }
            Err(e) => {
                diags.add_error(format!("Failed to get version: {}", e), None::<String>);
                ReadResponse {
                    state: None,
                    diagnostics: diags,
                }
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

    fn create_test_client(server_url: String) -> Client {
        Client::new(server_url, "test@pve!token=secret".to_string(), true).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn data_source_returns_version_successfully() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api2/json/version")
            .with_status(200)
            .with_body(
                r#"{
                "data": {
                    "version": "8.0.0",
                    "release": "8.0",
                    "repoid": "abc123"
                }
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let data_source = VersionDataSource::new(client);

        let request = ReadRequest {
            context: Context::new(),
            current_state: State::new(),
        };

        let response = data_source.read(request).await;
        assert!(response.diagnostics.errors.is_empty());
        assert!(response.state.is_some());

        let state = response.state.unwrap();
        match state.values.get("version").unwrap() {
            Dynamic::String(s) => assert_eq!(s, "8.0.0"),
            _ => panic!("Expected version to be a string"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn data_source_handles_api_failure() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api2/json/version")
            .with_status(500)
            .with_body(r#"{"errors":{"error":"Internal server error"}}"#)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let data_source = VersionDataSource::new(client);

        let request = ReadRequest {
            context: Context::new(),
            current_state: State::new(),
        };

        let response = data_source.read(request).await;
        assert!(!response.diagnostics.errors.is_empty());
        assert!(response.state.is_none());
    }

    #[tokio::test]
    async fn data_source_schema_has_correct_attributes() {
        let client = create_test_client("http://test".to_string());
        let data_source = VersionDataSource::new(client);

        let response = data_source
            .schema(SchemaRequest {
                context: Context::new(),
            })
            .await;

        assert!(response.schema.attributes.contains_key("id"));
        assert!(response.schema.attributes.contains_key("version"));
        assert!(response.schema.attributes.contains_key("release"));
        assert!(response.schema.attributes.contains_key("repoid"));
    }
}
