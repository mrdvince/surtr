//! Integration tests for access realm operations

use mockito::Server;
use proxmox::ProxmoxProvider;
use serial_test::serial;
use tfplug::context::Context;
use tfplug::data_source::ReadDataSourceRequest;
use tfplug::provider::{ConfigureProviderRequest, Provider};
use tfplug::types::{AttributePath, ClientCapabilities, DynamicValue};

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

    let mut config = DynamicValue::null();
    let _ = config.set_string(&AttributePath::new("endpoint"), server.url());
    let _ = config.set_string(
        &AttributePath::new("api_token"),
        "test@pve!test=secret123".to_string(),
    );
    let _ = config.set_bool(&AttributePath::new("insecure"), true);

    let config_request = ConfigureProviderRequest {
        terraform_version: "1.0.0".to_string(),
        config,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };

    let configure_response = provider.configure(Context::new(), config_request).await;
    assert!(configure_response.diagnostics.is_empty());
    assert!(configure_response.provider_data.is_some());

    // Use factory method to create data source
    let factories = provider.data_sources();
    let factory = factories.get("proxmox_version").unwrap();
    let mut version_ds = factory();

    // Configure the data source with provider data
    let configure_ds_request = tfplug::data_source::ConfigureDataSourceRequest {
        provider_data: configure_response.provider_data.clone(),
    };
    let configure_ds_response = version_ds
        .configure(Context::new(), configure_ds_request)
        .await;
    assert!(configure_ds_response.diagnostics.is_empty());

    let read_request = ReadDataSourceRequest {
        type_name: "proxmox_version".to_string(),
        config: DynamicValue::null(),
        provider_meta: None,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };

    let read_response = version_ds.read(Context::new(), read_request).await;

    assert!(read_response.diagnostics.is_empty());
    assert!(!read_response.state.is_null());

    let state = read_response.state;
    assert_eq!(
        state.get_string(&AttributePath::new("version")).unwrap(),
        "7.4.1"
    );
    assert_eq!(
        state.get_string(&AttributePath::new("release")).unwrap(),
        "7.4"
    );
    assert_eq!(
        state.get_string(&AttributePath::new("repoid")).unwrap(),
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

    // Create data source without configuring the provider first
    let factories = provider.data_sources();
    let factory = factories.get("proxmox_version").unwrap();
    let mut version_ds = factory();

    // Try to configure the data source without provider data
    let configure_ds_request = tfplug::data_source::ConfigureDataSourceRequest {
        provider_data: None,
    };
    let configure_ds_response = version_ds
        .configure(Context::new(), configure_ds_request)
        .await;

    // Should have diagnostics errors
    assert!(!configure_ds_response.diagnostics.is_empty());
    assert!(configure_ds_response.diagnostics[0]
        .summary
        .contains("No provider data"));

    // Try to read without provider data
    let read_request = ReadDataSourceRequest {
        type_name: "proxmox_version".to_string(),
        config: DynamicValue::null(),
        provider_meta: None,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };
    let read_response = version_ds.read(Context::new(), read_request).await;

    // Should fail with diagnostics errors
    assert!(!read_response.diagnostics.is_empty());
    assert!(read_response.diagnostics[0]
        .summary
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

    let mut config = DynamicValue::null();
    let _ = config.set_string(&AttributePath::new("endpoint"), server.url());
    let _ = config.set_string(
        &AttributePath::new("api_token"),
        "test@pve!test=secret123".to_string(),
    );
    let _ = config.set_bool(&AttributePath::new("insecure"), true);

    let config_request = ConfigureProviderRequest {
        terraform_version: "1.0.0".to_string(),
        config,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };

    let configure_response = provider.configure(Context::new(), config_request).await;
    assert!(configure_response.diagnostics.is_empty());
    assert!(configure_response.provider_data.is_some());

    // Test that we can create a realm resource through the factory
    let resource_factories = provider.resources();
    let realm_factory = resource_factories.get("proxmox_realm").unwrap();
    let mut _realm_resource = realm_factory();

    // Configure the resource with provider data
    let configure_res_request = tfplug::resource::ConfigureResourceRequest {
        provider_data: configure_response.provider_data.clone(),
    };
    let configure_res_response = _realm_resource
        .configure(Context::new(), configure_res_request)
        .await;
    assert!(configure_res_response.diagnostics.is_empty());

    // Test that we can create a version data source through the factory
    let ds_factories = provider.data_sources();
    let version_factory = ds_factories.get("proxmox_version").unwrap();
    let mut version_ds = version_factory();

    // Configure the data source with provider data
    let configure_ds_request = tfplug::data_source::ConfigureDataSourceRequest {
        provider_data: configure_response.provider_data.clone(),
    };
    let configure_ds_response = version_ds
        .configure(Context::new(), configure_ds_request)
        .await;
    assert!(configure_ds_response.diagnostics.is_empty());

    let read_request = ReadDataSourceRequest {
        type_name: "proxmox_version".to_string(),
        config: DynamicValue::null(),
        provider_meta: None,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };
    let read_response = version_ds.read(Context::new(), read_request).await;
    assert!(read_response.diagnostics.is_empty());
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

    let mut config = DynamicValue::null();
    let _ = config.set_string(&AttributePath::new("endpoint"), server.url());
    let _ = config.set_string(
        &AttributePath::new("api_token"),
        "invalid-token".to_string(),
    );
    let _ = config.set_bool(&AttributePath::new("insecure"), true);

    let config_request = ConfigureProviderRequest {
        terraform_version: "1.0.0".to_string(),
        config,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };

    let configure_response = provider.configure(Context::new(), config_request).await;
    assert!(configure_response.diagnostics.is_empty());
    assert!(configure_response.provider_data.is_some());

    // Create and configure data source
    let factories = provider.data_sources();
    let factory = factories.get("proxmox_version").unwrap();
    let mut version_ds = factory();

    let configure_ds_request = tfplug::data_source::ConfigureDataSourceRequest {
        provider_data: configure_response.provider_data.clone(),
    };
    let configure_ds_response = version_ds
        .configure(Context::new(), configure_ds_request)
        .await;
    assert!(configure_ds_response.diagnostics.is_empty());

    let read_request = ReadDataSourceRequest {
        type_name: "proxmox_version".to_string(),
        config: DynamicValue::null(),
        provider_meta: None,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };
    let read_response = version_ds.read(Context::new(), read_request).await;

    // Should fail with diagnostics errors due to 401 response
    assert!(!read_response.diagnostics.is_empty());
    assert!(read_response.diagnostics[0]
        .summary
        .contains("Failed to get version information"));
    assert!(read_response.state.is_null());
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
    let config_request = ConfigureProviderRequest {
        terraform_version: "1.0.0".to_string(),
        config: DynamicValue::null(),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };

    let configure_response = provider.configure(Context::new(), config_request).await;
    assert!(!configure_response.diagnostics.is_empty());
    assert!(configure_response.diagnostics[0]
        .summary
        .contains("Missing endpoint"));
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

    let mut config = DynamicValue::null();
    let _ = config.set_string(&AttributePath::new("endpoint"), server.url());
    let _ = config.set_string(
        &AttributePath::new("api_token"),
        "test@pve!test=secret123".to_string(),
    );
    // Explicitly set insecure to false
    let _ = config.set_bool(&AttributePath::new("insecure"), false);

    let config_request = ConfigureProviderRequest {
        terraform_version: "1.0.0".to_string(),
        config,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };

    let configure_response = provider.configure(Context::new(), config_request).await;
    assert!(configure_response.diagnostics.is_empty());
    assert!(configure_response.provider_data.is_some());

    // Create and configure data source
    let factories = provider.data_sources();
    let factory = factories.get("proxmox_version").unwrap();
    let mut version_ds = factory();

    let configure_ds_request = tfplug::data_source::ConfigureDataSourceRequest {
        provider_data: configure_response.provider_data.clone(),
    };
    let configure_ds_response = version_ds
        .configure(Context::new(), configure_ds_request)
        .await;
    assert!(configure_ds_response.diagnostics.is_empty());

    // In a real scenario with a self-signed cert, this would fail
    // But with mockito it should still work
    let read_request = ReadDataSourceRequest {
        type_name: "proxmox_version".to_string(),
        config: DynamicValue::null(),
        provider_meta: None,
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
    };
    let read_response = version_ds.read(Context::new(), read_request).await;

    assert!(read_response.diagnostics.is_empty());
    assert!(!read_response.state.is_null());

    let state = read_response.state;
    assert_eq!(
        state.get_string(&AttributePath::new("version")).unwrap(),
        "7.4.1"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn version_data_source_schema_is_correct() {
    let provider = ProxmoxProvider::new();
    let factories = provider.data_sources();
    let factory = factories.get("proxmox_version").unwrap();
    let version_ds = factory();

    // Get schema from the data source
    let schema_request = tfplug::data_source::DataSourceSchemaRequest {};
    let schema_response = version_ds.schema(Context::new(), schema_request).await;

    assert!(schema_response.diagnostics.is_empty());
    let schema = schema_response.schema;

    // Check expected attributes
    let attributes = &schema.block.attributes;
    let attribute_names: Vec<&str> = attributes.iter().map(|a| a.name.as_str()).collect();
    assert!(attribute_names.contains(&"id"));
    assert!(attribute_names.contains(&"version"));
    assert!(attribute_names.contains(&"release"));
    assert!(attribute_names.contains(&"repoid"));

    // All attributes should be computed
    for attr in attributes {
        assert!(attr.computed);
        assert!(!attr.required);
        assert!(!attr.optional);
    }
}
