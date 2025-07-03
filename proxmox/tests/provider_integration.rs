use mockito::Server;
use proxmox::ProxmoxProvider;
use serial_test::serial;
use std::collections::HashMap;
use tfplug::context::Context;
use tfplug::request::{ConfigureRequest, ReadRequest};
use tfplug::types::{Config, Dynamic, State};
use tfplug::ProviderV2;

#[tokio::test(flavor = "multi_thread")]
async fn provider_lifecycle_with_mock_server() {
    let mut server = Server::new_async().await;

    let _version_mock = server
        .mock("GET", "/api2/json/version")
        .with_header("authorization", "PVEAPIToken=test@pve!test=secret123")
        .with_body(r#"{"data":{"version":"7.4.1","release":"7.4","repoid":"12345"}}"#)
        .create_async()
        .await;

    let mut provider = ProxmoxProvider::new();

    let mut config_values = HashMap::new();
    config_values.insert("endpoint".to_string(), Dynamic::String(server.url()));
    config_values.insert(
        "api_token".to_string(),
        Dynamic::String("test@pve!test=secret123".to_string()),
    );
    config_values.insert("insecure".to_string(), Dynamic::Bool(true));

    let config_request = ConfigureRequest {
        context: Context::new(),
        config: Config {
            values: config_values,
        },
    };

    let configure_response = provider.configure(config_request).await;
    assert!(configure_response.diagnostics.errors.is_empty());

    // Use factory method to create data source
    let version_ds = provider
        .create_data_source("proxmox_version")
        .await
        .unwrap();

    let read_request = ReadRequest {
        context: Context::new(),
        current_state: State::new(),
    };

    let read_response = version_ds.read(read_request).await;

    assert!(read_response.diagnostics.errors.is_empty());
    assert!(read_response.state.is_some());

    let state = read_response.state.unwrap();
    assert_eq!(
        state.values.get("version").unwrap().as_string().unwrap(),
        "7.4.1"
    );
    assert_eq!(
        state.values.get("release").unwrap().as_string().unwrap(),
        "7.4"
    );
    assert_eq!(
        state.values.get("repoid").unwrap().as_string().unwrap(),
        "12345"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn version_data_source_requires_configured_provider() {
    let mut server = Server::new_async().await;

    let _version_mock = server
        .mock("GET", "/api2/json/version")
        .with_header("authorization", "PVEAPIToken=test@pve!test=secret123")
        .with_body(r#"{"data":{"version":"8.0.0","release":"8.0","repoid":"67890"}}"#)
        .create_async()
        .await;

    let provider = ProxmoxProvider::new();

    // Try to create data source without configuring the provider first
    let result = provider.create_data_source("proxmox_version").await;

    // Should fail because provider not configured
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("Provider not configured"));
}

#[tokio::test(flavor = "multi_thread")]
async fn realm_resource_lifecycle() {
    let mut server = Server::new_async().await;

    let _create_mock = server
        .mock("POST", "/api2/json/access/domains")
        .with_header("authorization", "PVEAPIToken=test@pve!test=secret123")
        .with_body(r#"{"data":null}"#)
        .create_async()
        .await;

    let _version_mock = server
        .mock("GET", "/api2/json/version")
        .with_header("authorization", "PVEAPIToken=test@pve!test=secret123")
        .with_body(r#"{"data":{"version":"7.4.1","release":"7.4","repoid":"12345"}}"#)
        .create_async()
        .await;

    let mut provider = ProxmoxProvider::new();

    let mut config_values = HashMap::new();
    config_values.insert("endpoint".to_string(), Dynamic::String(server.url()));
    config_values.insert(
        "api_token".to_string(),
        Dynamic::String("test@pve!test=secret123".to_string()),
    );
    config_values.insert("insecure".to_string(), Dynamic::Bool(true));

    let config_request = ConfigureRequest {
        context: Context::new(),
        config: Config {
            values: config_values,
        },
    };

    let configure_response = provider.configure(config_request).await;
    assert!(configure_response.diagnostics.errors.is_empty());

    // Test that we can create a realm resource through the factory
    let _realm_resource = provider.create_resource("proxmox_realm").await.unwrap();

    // Test that we can create a version data source through the factory
    let version_ds = provider
        .create_data_source("proxmox_version")
        .await
        .unwrap();
    let read_request = ReadRequest {
        context: Context::new(),
        current_state: State::new(),
    };
    let read_response = version_ds.read(read_request).await;
    assert!(read_response.diagnostics.errors.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn handles_api_errors_gracefully() {
    let mut server = Server::new_async().await;

    let _version_mock = server
        .mock("GET", "/api2/json/version")
        .with_status(401)
        .with_body(r#"{"errors":{"authentication":"Invalid token"}}"#)
        .create_async()
        .await;

    let mut provider = ProxmoxProvider::new();

    let mut config_values = HashMap::new();
    config_values.insert("endpoint".to_string(), Dynamic::String(server.url()));
    config_values.insert(
        "api_token".to_string(),
        Dynamic::String("invalid-token".to_string()),
    );
    config_values.insert("insecure".to_string(), Dynamic::Bool(true));

    let config_request = ConfigureRequest {
        context: Context::new(),
        config: Config {
            values: config_values,
        },
    };

    let configure_response = provider.configure(config_request).await;
    assert!(configure_response.diagnostics.errors.is_empty());

    let version_ds = provider
        .create_data_source("proxmox_version")
        .await
        .unwrap();
    let read_request = ReadRequest {
        context: Context::new(),
        current_state: State::new(),
    };
    let read_response = version_ds.read(read_request).await;

    // Should fail with diagnostics errors due to 401 response
    assert!(!read_response.diagnostics.errors.is_empty());
    assert!(read_response.state.is_none());
}

#[tokio::test]
#[serial]
async fn provider_configuration_validation() {
    // Clear environment variables first to ensure clean test
    std::env::remove_var("PROXMOX_ENDPOINT");
    std::env::remove_var("PROXMOX_API_TOKEN");
    std::env::remove_var("PROXMOX_INSECURE");

    let mut provider = ProxmoxProvider::new();

    // Missing required fields
    let config_request = ConfigureRequest {
        context: Context::new(),
        config: Config {
            values: HashMap::new(),
        },
    };

    let configure_response = provider.configure(config_request).await;
    assert!(!configure_response.diagnostics.errors.is_empty());
    assert!(configure_response.diagnostics.errors[0]
        .summary
        .contains("endpoint is required"));
}

#[tokio::test(flavor = "multi_thread")]
async fn respects_insecure_tls_setting() {
    let mut server = Server::new_async().await;

    let _version_mock = server
        .mock("GET", "/api2/json/version")
        .with_header("authorization", "PVEAPIToken=test@pve!test=secret123")
        .with_body(r#"{"data":{"version":"7.4.1","release":"7.4","repoid":"12345"}}"#)
        .create_async()
        .await;

    let mut provider = ProxmoxProvider::new();

    let mut config_values = HashMap::new();
    config_values.insert("endpoint".to_string(), Dynamic::String(server.url()));
    config_values.insert(
        "api_token".to_string(),
        Dynamic::String("test@pve!test=secret123".to_string()),
    );
    // Explicitly set insecure to false
    config_values.insert("insecure".to_string(), Dynamic::Bool(false));

    let config_request = ConfigureRequest {
        context: Context::new(),
        config: Config {
            values: config_values,
        },
    };

    let configure_response = provider.configure(config_request).await;
    assert!(configure_response.diagnostics.errors.is_empty());

    let version_ds = provider
        .create_data_source("proxmox_version")
        .await
        .unwrap();

    // In a real scenario with a self-signed cert, this would fail
    // But with mockito it should still work
    let read_request = ReadRequest {
        context: Context::new(),
        current_state: State::new(),
    };
    let read_response = version_ds.read(read_request).await;

    assert!(read_response.diagnostics.errors.is_empty());
    assert!(read_response.state.is_some());
    let state = read_response.state.unwrap();
    assert_eq!(
        state.values.get("version").unwrap().as_string().unwrap(),
        "7.4.1"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn version_data_source_schema_is_correct() {
    let provider = ProxmoxProvider::new();
    let schemas = provider.data_source_schemas().await;

    assert!(schemas.contains_key("proxmox_version"));
    let version_schema = &schemas["proxmox_version"];

    // Check expected attributes
    assert!(version_schema.attributes.contains_key("id"));
    assert!(version_schema.attributes.contains_key("version"));
    assert!(version_schema.attributes.contains_key("release"));
    assert!(version_schema.attributes.contains_key("repoid"));

    // All attributes should be computed
    for (_, attr) in &version_schema.attributes {
        assert!(attr.computed);
        assert!(!attr.required);
        assert!(!attr.optional);
    }
}
