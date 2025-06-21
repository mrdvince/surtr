use mockito::Server;
use proxmox::ProxmoxProvider;
use std::collections::HashMap;
use tfplug::{Config, Dynamic, Provider};

#[tokio::test(flavor = "multi_thread")]
async fn provider_lifecycle_with_mock_server() {
    let mut server = Server::new_async().await;
    
    let _version_mock = server.mock("GET", "/api2/json/version")
        .with_header("authorization", "PVEAPIToken=test@pve!test=secret123")
        .with_body(r#"{"data":{"version":"7.4.1","release":"7.4","repoid":"12345"}}"#)
        .create_async()
        .await;

    let mut provider = ProxmoxProvider::new();
    
    let mut config_values = HashMap::new();
    config_values.insert("endpoint".to_string(), Dynamic::String(server.url()));
    config_values.insert("api_token".to_string(), Dynamic::String("test@pve!test=secret123".to_string()));
    config_values.insert("insecure".to_string(), Dynamic::Bool(true));
    
    let config = Config { values: config_values };
    
    let diags = provider.configure(config).unwrap();
    assert!(!diags.has_errors());
    
    let data_sources = provider.get_data_sources();
    let version_ds = data_sources.get("proxmox_version").unwrap();
    let (state, read_diags) = version_ds.read(Config { values: HashMap::new() }).unwrap();
    
    assert!(!read_diags.has_errors());
    assert_eq!(state.values.get("version").unwrap().as_string().unwrap(), "7.4.1");
    assert_eq!(state.values.get("release").unwrap().as_string().unwrap(), "7.4");
    assert_eq!(state.values.get("repoid").unwrap().as_string().unwrap(), "12345");
    assert_eq!(state.values.get("id").unwrap().as_string().unwrap(), "proxmox_version");
}

#[tokio::test]
async fn provider_handles_missing_endpoint() {
    let mut provider = ProxmoxProvider::new();
    
    let mut config_values = HashMap::new();
    config_values.insert("api_token".to_string(), Dynamic::String("token".to_string()));
    
    let config = Config { values: config_values };
    
    let result = provider.configure(config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("endpoint is required"));
}

#[tokio::test]
async fn provider_handles_missing_api_token() {
    let mut provider = ProxmoxProvider::new();
    
    let mut config_values = HashMap::new();
    config_values.insert("endpoint".to_string(), Dynamic::String("https://pve.example.com".to_string()));
    
    let config = Config { values: config_values };
    
    let result = provider.configure(config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("api_token is required"));
}

#[tokio::test]
async fn unconfigured_provider_returns_empty_data_sources() {
    let provider = ProxmoxProvider::new();
    let data_sources = provider.get_data_sources();
    assert!(data_sources.is_empty());
}

#[tokio::test]
async fn provider_schema_available_without_configuration() {
    let provider = ProxmoxProvider::new();
    let schemas = provider.get_schema();
    
    assert!(schemas.contains_key("proxmox_version"));
    let version_schema = &schemas["proxmox_version"];
    assert!(version_schema.attributes.contains_key("version"));
    assert!(version_schema.attributes.contains_key("release"));
    assert!(version_schema.attributes.contains_key("repoid"));
}