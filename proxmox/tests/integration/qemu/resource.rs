use mockito::{Matcher, Server};
use proxmox::api::Client;
use proxmox::resources::nodes::QemuVmResource;
use proxmox::ProxmoxProviderData;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tfplug::context::Context;
use tfplug::resource::{
    ConfigureResourceRequest, CreateResourceRequest, DeleteResourceRequest,
    ImportResourceStateRequest, ReadResourceRequest, Resource, ResourceMetadataRequest,
    ResourceSchemaRequest, ResourceWithConfigure, ResourceWithImportState, UpdateResourceRequest,
};
use tfplug::types::{AttributePath, ClientCapabilities, Dynamic, DynamicValue};

fn create_test_provider_data(server_url: &str) -> ProxmoxProviderData {
    let client = Client::new(server_url, "test@pam!test=secret", true).unwrap();
    ProxmoxProviderData {
        client: Arc::new(client),
    }
}

fn create_test_dynamic_value() -> DynamicValue {
    let mut obj = std::collections::HashMap::new();
    obj.insert(
        "target_node".to_string(),
        Dynamic::String("pve".to_string()),
    );
    obj.insert("vmid".to_string(), Dynamic::Number(100.0));
    obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
    obj.insert("memory".to_string(), Dynamic::Number(2048.0));
    obj.insert("cores".to_string(), Dynamic::Number(2.0));
    obj.insert("sockets".to_string(), Dynamic::Number(1.0));
    DynamicValue::new(Dynamic::Map(obj))
}

#[tokio::test]
async fn test_resource_metadata() {
    let resource = QemuVmResource::new();
    let ctx = Context::new();
    let request = ResourceMetadataRequest {};
    let response = resource.metadata(ctx, request).await;

    assert_eq!(response.type_name, "proxmox_qemu_vm");
}

#[tokio::test]
async fn test_resource_schema() {
    let resource = QemuVmResource::new();
    let ctx = Context::new();
    let request = ResourceSchemaRequest {};
    let response = resource.schema(ctx, request).await;

    assert!(response.diagnostics.is_empty());
    assert_eq!(response.schema.version, 0);

    let attrs = &response.schema.block.attributes;
    assert!(attrs.iter().any(|a| a.name == "target_node" && a.required));
    assert!(attrs.iter().any(|a| a.name == "vmid" && a.required));
    assert!(attrs.iter().any(|a| a.name == "name" && a.required));
    assert!(attrs.iter().any(|a| a.name == "cores" && !a.required));
    assert!(attrs.iter().any(|a| a.name == "memory" && !a.required));
    assert!(attrs.iter().any(|a| a.name == "cipassword" && a.sensitive));
}

#[tokio::test]
async fn test_create_without_provider_data() {
    let resource = QemuVmResource::new();
    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: create_test_dynamic_value(),
        planned_state: create_test_dynamic_value(),
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    assert_eq!(response.diagnostics.len(), 1);
    assert!(response.diagnostics[0]
        .summary
        .contains("Provider not configured"));
}

#[tokio::test]
async fn test_create_successful() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{"vmid":100,"name":"test-vm","memory":2048,"cores":2,"sockets":1}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: create_test_dynamic_value(),
        planned_state: create_test_dynamic_value(),
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());
}

#[tokio::test]
async fn test_read_without_provider_data() {
    let resource = QemuVmResource::new();
    let ctx = Context::new();
    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state: create_test_dynamic_value(),
        private: vec![],
        provider_meta: Some(DynamicValue::null()),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert_eq!(response.diagnostics.len(), 1);
    assert!(response.diagnostics[0]
        .summary
        .contains("Provider not configured"));
}

#[tokio::test]
async fn test_read_successful() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm-updated",
                    "cores": 4,
                    "memory": 4096,
                    "sockets": 2,
                    "cpu": "host",
                    "ostype": "l26"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state: create_test_dynamic_value(),
        private: vec![],
        provider_meta: Some(DynamicValue::null()),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert!(response.diagnostics.is_empty());
    assert!(response.new_state.is_some());

    let new_state = response.new_state.unwrap();
    assert_eq!(
        new_state.get_string(&AttributePath::new("name")).unwrap(),
        "test-vm-updated"
    );
    assert_eq!(
        new_state.get_number(&AttributePath::new("cores")).unwrap(),
        4.0
    );
    assert_eq!(
        new_state.get_number(&AttributePath::new("memory")).unwrap(),
        4096.0
    );
    assert_eq!(
        new_state
            .get_number(&AttributePath::new("sockets"))
            .unwrap(),
        2.0
    );
}

#[tokio::test]
async fn test_read_vm_not_found() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": null,
                "errors": {
                    "vmid": "VM 100 not found"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state: create_test_dynamic_value(),
        private: vec![],
        provider_meta: Some(DynamicValue::null()),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert!(response.new_state.is_none());
}

#[tokio::test]
async fn test_read_normalizes_tags() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "vmid": 100,
                    "name": "test-vm",
                    "cores": 2,
                    "sockets": 1,
                    "memory": 2048,
                    "tags": "web;production;test"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let mut current_state = create_test_dynamic_value();
    // Add tags to current state so they will be populated
    current_state
        .set_string(&AttributePath::new("tags"), "web,production".to_string())
        .unwrap();

    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state,
        private: vec![],
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        provider_meta: None,
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert!(response.diagnostics.is_empty());
    assert!(response.new_state.is_some());

    let new_state = response.new_state.unwrap();
    let tags = new_state.get_string(&AttributePath::new("tags")).unwrap();
    assert_eq!(tags, "web,production,test");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_read_preserves_boot_order() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "vmid": 100,
                    "name": "test-vm",
                    "cores": 2,
                    "sockets": 1,
                    "memory": 2048,
                    "boot": "order=scsi0;ide2;net0"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state: create_test_dynamic_value(),
        private: vec![],
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        provider_meta: None,
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert!(response.diagnostics.is_empty());
    assert!(response.new_state.is_some());

    let new_state = response.new_state.unwrap();
    // Boot should not be set if it wasn't in the current state
    assert!(new_state.get_string(&AttributePath::new("boot")).is_err());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_read_normalizes_network_macs() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "vmid": 100,
                    "name": "test-vm",
                    "cores": 2,
                    "sockets": 1,
                    "memory": 2048,
                    "net0": "virtio=BA:88:CB:76:75:D6,bridge=vmbr0,firewall=0,tag=30"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state: create_test_dynamic_value(),
        private: vec![],
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        provider_meta: None,
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert!(response.diagnostics.is_empty());
    assert!(response.new_state.is_some());

    let new_state = response.new_state.unwrap();
    let net0 = new_state.get_string(&AttributePath::new("net0")).unwrap();
    // When current config doesn't have a MAC, it should be stripped from the response
    assert_eq!(net0, "virtio,bridge=vmbr0,firewall=0,tag=30");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_update_successful() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/api2/json/nodes/pve/qemu/100/config")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(serde_json::json!({
            "name": "test-vm",
            "memory": 4096,
            "cores": 4,
            "sockets": 1
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": null
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();

    let mut updated_obj = std::collections::HashMap::new();
    updated_obj.insert(
        "target_node".to_string(),
        Dynamic::String("pve".to_string()),
    );
    updated_obj.insert("vmid".to_string(), Dynamic::Number(100.0));
    updated_obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
    updated_obj.insert("memory".to_string(), Dynamic::Number(4096.0));
    updated_obj.insert("cores".to_string(), Dynamic::Number(4.0));
    updated_obj.insert("sockets".to_string(), Dynamic::Number(1.0));

    let request = UpdateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: DynamicValue::new(Dynamic::Map(updated_obj.clone())),
        planned_state: DynamicValue::new(Dynamic::Map(updated_obj.clone())),
        prior_state: create_test_dynamic_value(),
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
        planned_identity: None,
    };

    let response = resource.update(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());
}

#[tokio::test]
async fn test_delete_successful() {
    let mut server = Server::new_async().await;

    // Mock the status check - VM is stopped
    let _m_status = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/status/current")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "status": "stopped"
                }
            }"#,
        )
        .create_async()
        .await;

    let _m_delete = server
        .mock("DELETE", "/api2/json/nodes/pve/qemu/100")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmdestroy:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = DeleteResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        prior_state: create_test_dynamic_value(),
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.delete(ctx, request).await;
    assert!(response.diagnostics.is_empty());
}

#[tokio::test]
async fn test_delete_running_vm() {
    let mut server = Server::new_async().await;

    // Mock the status check - VM is running
    let _m_status = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/status/current")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "status": "running"
                }
            }"#,
        )
        .create_async()
        .await;

    // Mock the stop call
    let _m_stop = server
        .mock("POST", "/api2/json/nodes/pve/qemu/100/status/stop")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmstop:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m_delete = server
        .mock("DELETE", "/api2/json/nodes/pve/qemu/100")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmdestroy:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = DeleteResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        prior_state: create_test_dynamic_value(),
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.delete(ctx, request).await;
    assert!(response.diagnostics.is_empty());

    // Note: In a real test we'd need to handle the 5-second sleep, but mockito
    // doesn't actually execute it when mocking the HTTP calls
}

#[tokio::test]
async fn test_import_state() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "imported-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();
    let request = ImportResourceStateRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        id: "pve/100".to_string(),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        identity: None,
    };

    let response = resource.import_state(ctx, request).await;
    assert!(response.diagnostics.is_empty());
    assert_eq!(response.imported_resources.len(), 1);

    let imported = &response.imported_resources[0];
    assert_eq!(imported.type_name, "proxmox_qemu_vm");
    assert_eq!(
        imported
            .state
            .get_string(&AttributePath::new("target_node"))
            .unwrap(),
        "pve"
    );
    assert_eq!(
        imported
            .state
            .get_number(&AttributePath::new("vmid"))
            .unwrap(),
        100.0
    );
    assert_eq!(
        imported
            .state
            .get_string(&AttributePath::new("name"))
            .unwrap(),
        "imported-vm"
    );
}

#[tokio::test]
async fn test_import_state_invalid_id() {
    let resource = QemuVmResource::new();
    let ctx = Context::new();
    let request = ImportResourceStateRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        id: "invalid-format".to_string(),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        identity: None,
    };

    let response = resource.import_state(ctx, request).await;
    assert_eq!(response.diagnostics.len(), 1);
    assert!(response.diagnostics[0]
        .summary
        .contains("Invalid import ID"));
}

#[tokio::test]
async fn test_configure_resource() {
    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data("https://test.example.com:8006");
    let ctx = Context::new();

    let request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };

    let response = resource.configure(ctx, request).await;
    assert!(response.diagnostics.is_empty());
}

#[tokio::test]
async fn test_create_populates_network_interfaces() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "net0": "virtio=BC:24:11:AA:BB:CC,bridge=vmbr0,firewall=1"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();
    config
        .set_string(
            &AttributePath::new("net0"),
            "virtio,bridge=vmbr0,firewall=1".to_string(),
        )
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    // Verify network interface was populated with normalized value
    let net0 = response
        .new_state
        .get_string(&AttributePath::new("net0"))
        .unwrap();
    assert_eq!(net0, "virtio,bridge=vmbr0,firewall=1");
}

#[tokio::test]
async fn test_create_vm_with_network_blocks() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1,
              "net0": "virtio,bridge=vmbr0,firewall=1,tag=10",
              "net1": "e1000,bridge=vmbr1,firewall=0"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "net0": "virtio=BC:24:11:AA:BB:CC,bridge=vmbr0,firewall=1,tag=10",
                    "net1": "e1000=DE:AD:BE:EF:00:01,bridge=vmbr1,firewall=0"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();

    let mut net0 = HashMap::new();
    net0.insert("id".to_string(), Dynamic::Number(0.0));
    net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
    net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
    net0.insert("firewall".to_string(), Dynamic::Bool(true));
    net0.insert("tag".to_string(), Dynamic::Number(10.0));

    let mut net1 = HashMap::new();
    net1.insert("id".to_string(), Dynamic::Number(1.0));
    net1.insert("model".to_string(), Dynamic::String("e1000".to_string()));
    net1.insert("bridge".to_string(), Dynamic::String("vmbr1".to_string()));
    net1.insert("firewall".to_string(), Dynamic::Bool(false));

    config
        .set_list(
            &AttributePath::new("network"),
            vec![Dynamic::Map(net0), Dynamic::Map(net1)],
        )
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    let networks = response
        .new_state
        .get_list(&AttributePath::new("network"))
        .unwrap();
    assert_eq!(networks.len(), 2);

    match &networks[0] {
        Dynamic::Map(net0_block) => {
            assert_eq!(net0_block.get("id").unwrap(), &Dynamic::Number(0.0));
            assert_eq!(
                net0_block.get("model").unwrap(),
                &Dynamic::String("virtio".to_string())
            );
            assert_eq!(
                net0_block.get("bridge").unwrap(),
                &Dynamic::String("vmbr0".to_string())
            );
            assert_eq!(net0_block.get("firewall").unwrap(), &Dynamic::Bool(true));
            assert_eq!(net0_block.get("tag").unwrap(), &Dynamic::Number(10.0));
            // MAC address is only present if provided in config or after reading from API
            // Since we're returning planned state, it won't be present unless explicitly set
        }
        _ => panic!("Expected network[0] to be a map"),
    }

    match &networks[1] {
        Dynamic::Map(net1_block) => {
            assert_eq!(net1_block.get("id").unwrap(), &Dynamic::Number(1.0));
            assert_eq!(
                net1_block.get("model").unwrap(),
                &Dynamic::String("e1000".to_string())
            );
            assert_eq!(
                net1_block.get("bridge").unwrap(),
                &Dynamic::String("vmbr1".to_string())
            );
            assert_eq!(net1_block.get("firewall").unwrap(), &Dynamic::Bool(false));
            // MAC address is only present if provided in config or after reading from API
            // Since we're returning planned state, it won't be present unless explicitly set
        }
        _ => panic!("Expected network[1] to be a map"),
    }
}

#[tokio::test]
async fn test_create_vm_with_disk_blocks() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1,
              "scsihw": "virtio-scsi-single",
              "scsi0": "local-lvm:10,format=raw,iothread=1,ssd=1",
              "scsi1": "local-lvm:20,format=qcow2",
              "virtio0": "local-lvm:30,discard=on",
              "ide3": "local:cloudinit"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "scsi0": "local-lvm:vm-100-disk-0,format=raw,iothread=1,size=10G,ssd=1",
                    "scsi1": "local-lvm:vm-100-disk-1,format=qcow2,size=20G",
                    "virtio0": "local-lvm:vm-100-disk-2,discard=on,size=30G",
                    "ide3": "local:cloudinit"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();
    config
        .set_string(
            &AttributePath::new("scsihw"),
            "virtio-scsi-single".to_string(),
        )
        .unwrap();

    let mut disk0 = HashMap::new();
    disk0.insert("slot".to_string(), Dynamic::String("scsi0".to_string()));
    disk0.insert("type".to_string(), Dynamic::String("scsi".to_string()));
    disk0.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    disk0.insert("size".to_string(), Dynamic::String("10G".to_string()));
    disk0.insert("format".to_string(), Dynamic::String("raw".to_string()));
    disk0.insert("iothread".to_string(), Dynamic::Bool(true));
    disk0.insert("emulatessd".to_string(), Dynamic::Bool(true));

    let mut disk1 = HashMap::new();
    disk1.insert("slot".to_string(), Dynamic::String("scsi1".to_string()));
    disk1.insert("type".to_string(), Dynamic::String("scsi".to_string()));
    disk1.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    disk1.insert("size".to_string(), Dynamic::String("20G".to_string()));
    disk1.insert("format".to_string(), Dynamic::String("qcow2".to_string()));

    let mut disk2 = HashMap::new();
    disk2.insert("slot".to_string(), Dynamic::String("virtio0".to_string()));
    disk2.insert("type".to_string(), Dynamic::String("virtio".to_string()));
    disk2.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    disk2.insert("size".to_string(), Dynamic::String("30G".to_string()));
    disk2.insert("discard".to_string(), Dynamic::Bool(true));

    config
        .set_list(
            &AttributePath::new("disk"),
            vec![
                Dynamic::Map(disk0),
                Dynamic::Map(disk1),
                Dynamic::Map(disk2),
            ],
        )
        .unwrap();

    // cloudinit should be set as a cloudinit_drive block
    let mut cloudinit = HashMap::new();
    cloudinit.insert("slot".to_string(), Dynamic::String("ide3".to_string()));
    cloudinit.insert("storage".to_string(), Dynamic::String("local".to_string()));
    config
        .set_list(
            &AttributePath::new("cloudinit_drive"),
            vec![Dynamic::Map(cloudinit)],
        )
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    // Check disk blocks - we should have 3 disks (scsi0, scsi1, virtio0)
    let disks = response
        .new_state
        .get_list(&AttributePath::new("disk"))
        .unwrap();
    assert!(disks.len() >= 3);

    match &disks[0] {
        Dynamic::Map(disk0_block) => {
            assert_eq!(
                disk0_block.get("slot").unwrap(),
                &Dynamic::String("scsi0".to_string())
            );
            assert_eq!(
                disk0_block.get("storage").unwrap(),
                &Dynamic::String("local-lvm".to_string())
            );
            assert_eq!(
                disk0_block.get("size").unwrap(),
                &Dynamic::String("10G".to_string())
            );
            assert_eq!(
                disk0_block.get("format").unwrap(),
                &Dynamic::String("raw".to_string())
            );
            assert_eq!(disk0_block.get("iothread").unwrap(), &Dynamic::Bool(true));
            assert_eq!(disk0_block.get("emulatessd").unwrap(), &Dynamic::Bool(true));
        }
        _ => panic!("Expected disk[0] to be a map"),
    }

    // Check that ide2 is preserved as a string attribute
    // Verify cloudinit_drive blocks were populated correctly
    let cloudinit_drives = response
        .new_state
        .get_list(&AttributePath::new("cloudinit_drive"))
        .unwrap();
    assert_eq!(cloudinit_drives.len(), 1);
}

#[tokio::test]
async fn test_create_vm_with_efidisk_block() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1,
              "bios": "ovmf",
              "efidisk0": "local-lvm:1,efitype=4m"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "bios": "ovmf",
                    "efidisk0": "local-lvm:vm-100-disk-0,efitype=4m,format=qcow2,pre-enrolled-keys=1,size=1M"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();
    config
        .set_string(&AttributePath::new("bios"), "ovmf".to_string())
        .unwrap();

    let mut efidisk = HashMap::new();
    efidisk.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    efidisk.insert("efitype".to_string(), Dynamic::String("4m".to_string()));

    config
        .set_list(&AttributePath::new("efidisk"), vec![Dynamic::Map(efidisk)])
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    let efidisk_list = response
        .new_state
        .get_list(&AttributePath::new("efidisk"))
        .unwrap();
    assert_eq!(efidisk_list.len(), 1);

    match &efidisk_list[0] {
        Dynamic::Map(efidisk_block) => {
            assert_eq!(
                efidisk_block.get("storage").unwrap(),
                &Dynamic::String("local-lvm".to_string())
            );
            assert_eq!(
                efidisk_block.get("efitype").unwrap(),
                &Dynamic::String("4m".to_string())
            );
        }
        _ => panic!("Expected efidisk[0] to be a map"),
    }
}

// TODO: Enable this test when cloud-init fields are added to CreateQemuRequest
#[ignore]
#[tokio::test]
async fn test_create_vm_with_cloudinit_blocks() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "ciuser": "ubuntu",
                    "sshkeys": "ssh-rsa AAAAB3... user@example.com",
                    "ipconfig0": "ip=192.168.1.100/24,gw=192.168.1.1",
                    "ipconfig1": "ip=10.0.0.100/24"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();

    let mut cloudinit = HashMap::new();
    cloudinit.insert("user".to_string(), Dynamic::String("ubuntu".to_string()));
    cloudinit.insert(
        "password".to_string(),
        Dynamic::String("secret123".to_string()),
    );
    cloudinit.insert(
        "ssh_keys".to_string(),
        Dynamic::String("ssh-rsa AAAAB3... user@example.com".to_string()),
    );

    let mut ipconfig0 = HashMap::new();
    ipconfig0.insert("id".to_string(), Dynamic::Number(0.0));
    ipconfig0.insert(
        "ipv4".to_string(),
        Dynamic::String("192.168.1.100/24".to_string()),
    );
    ipconfig0.insert(
        "gateway".to_string(),
        Dynamic::String("192.168.1.1".to_string()),
    );

    let mut ipconfig1 = HashMap::new();
    ipconfig1.insert("id".to_string(), Dynamic::Number(1.0));
    ipconfig1.insert(
        "ipv4".to_string(),
        Dynamic::String("10.0.0.100/24".to_string()),
    );

    cloudinit.insert(
        "ipconfig".to_string(),
        Dynamic::List(vec![Dynamic::Map(ipconfig0), Dynamic::Map(ipconfig1)]),
    );

    config
        .set_map(&AttributePath::new("cloudinit"), cloudinit)
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    let cloudinit_block = response
        .new_state
        .get_map(&AttributePath::new("cloudinit"))
        .unwrap();
    assert_eq!(
        cloudinit_block.get("user").unwrap(),
        &Dynamic::String("ubuntu".to_string())
    );
    assert_eq!(
        cloudinit_block.get("password").unwrap(),
        &Dynamic::String("secret123".to_string())
    );
    assert_eq!(
        cloudinit_block.get("ssh_keys").unwrap(),
        &Dynamic::String("ssh-rsa AAAAB3... user@example.com".to_string())
    );

    match cloudinit_block.get("ipconfig").unwrap() {
        Dynamic::List(ipconfigs) => {
            assert_eq!(ipconfigs.len(), 2);
        }
        _ => panic!("Expected ipconfig to be a list"),
    }
}

#[tokio::test]
async fn test_update_vm_with_nested_blocks() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/api2/json/nodes/pve/qemu/100/config")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "name": "test-vm",
              "memory": 4096,
              "cores": 4,
              "sockets": 1,
              "scsihw": "virtio-scsi-single",
              "net0": "virtio,bridge=vmbr0,firewall=1,tag=20",
              "scsi0": "local-lvm:20,format=raw,iothread=1"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": null
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();

    let mut updated_obj = std::collections::HashMap::new();
    updated_obj.insert(
        "target_node".to_string(),
        Dynamic::String("pve".to_string()),
    );
    updated_obj.insert("vmid".to_string(), Dynamic::Number(100.0));
    updated_obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
    updated_obj.insert("memory".to_string(), Dynamic::Number(4096.0));
    updated_obj.insert("cores".to_string(), Dynamic::Number(4.0));
    updated_obj.insert("sockets".to_string(), Dynamic::Number(1.0));
    updated_obj.insert(
        "scsihw".to_string(),
        Dynamic::String("virtio-scsi-single".to_string()),
    );

    let mut net0 = HashMap::new();
    net0.insert("id".to_string(), Dynamic::Number(0.0));
    net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
    net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
    net0.insert("firewall".to_string(), Dynamic::Bool(true));
    net0.insert("tag".to_string(), Dynamic::Number(20.0));
    updated_obj.insert(
        "network".to_string(),
        Dynamic::List(vec![Dynamic::Map(net0)]),
    );

    let mut disk0 = HashMap::new();
    disk0.insert("slot".to_string(), Dynamic::String("scsi0".to_string()));
    disk0.insert("type".to_string(), Dynamic::String("scsi".to_string()));
    disk0.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    disk0.insert("size".to_string(), Dynamic::String("20G".to_string()));
    disk0.insert("format".to_string(), Dynamic::String("raw".to_string()));
    disk0.insert("iothread".to_string(), Dynamic::Bool(true));
    updated_obj.insert("disk".to_string(), Dynamic::List(vec![Dynamic::Map(disk0)]));

    let request = UpdateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: DynamicValue::new(Dynamic::Map(updated_obj.clone())),
        planned_state: DynamicValue::new(Dynamic::Map(updated_obj.clone())),
        prior_state: create_test_dynamic_value(),
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
        planned_identity: None,
    };

    let response = resource.update(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());
}

#[tokio::test]
async fn test_read_vm_with_nested_blocks() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 4,
                    "memory": 4096,
                    "sockets": 2,
                    "vmid": 100,
                    "net0": "virtio=BC:24:11:AA:BB:CC,bridge=vmbr0,firewall=1,tag=10",
                    "net1": "e1000=DE:AD:BE:EF:00:01,bridge=vmbr1",
                    "scsi0": "local-lvm:vm-100-disk-0,format=raw,iothread=1,size=10G,ssd=1",
                    "virtio0": "local-lvm:vm-100-disk-1,discard=on,size=20G",
                    "efidisk0": "local-lvm:vm-100-disk-2,efitype=4m,format=qcow2,size=1M",
                    "ciuser": "ubuntu",
                    "ipconfig0": "ip=192.168.1.100/24,gw=192.168.1.1",
                    "ipconfig1": "ip=10.0.0.100/24"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let ctx = Context::new();

    let mut current_state = create_test_dynamic_value();

    let mut net0 = HashMap::new();
    net0.insert("id".to_string(), Dynamic::Number(0.0));
    net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
    net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
    net0.insert("firewall".to_string(), Dynamic::Bool(true));
    net0.insert("tag".to_string(), Dynamic::Number(10.0));

    let mut net1 = HashMap::new();
    net1.insert("id".to_string(), Dynamic::Number(1.0));
    net1.insert("model".to_string(), Dynamic::String("e1000".to_string()));
    net1.insert("bridge".to_string(), Dynamic::String("vmbr1".to_string()));

    current_state
        .set_list(
            &AttributePath::new("network"),
            vec![Dynamic::Map(net0), Dynamic::Map(net1)],
        )
        .unwrap();

    let request = ReadResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        current_state,
        private: vec![],
        provider_meta: Some(DynamicValue::null()),
        client_capabilities: ClientCapabilities {
            deferral_allowed: false,
            write_only_attributes_allowed: false,
        },
        current_identity: None,
    };

    let response = resource.read(ctx, request).await;
    assert!(response.diagnostics.is_empty());
    assert!(response.new_state.is_some());

    let new_state = response.new_state.unwrap();

    let networks = new_state.get_list(&AttributePath::new("network")).unwrap();
    assert_eq!(networks.len(), 2);

    match &networks[0] {
        Dynamic::Map(net0_block) => {
            assert_eq!(
                net0_block.get("model").unwrap(),
                &Dynamic::String("virtio".to_string())
            );
            assert_eq!(
                net0_block.get("bridge").unwrap(),
                &Dynamic::String("vmbr0".to_string())
            );
            assert_eq!(net0_block.get("firewall").unwrap(), &Dynamic::Bool(true));
            assert_eq!(net0_block.get("tag").unwrap(), &Dynamic::Number(10.0));
            // MAC address is only present if provided in config or after reading from API
            // Since we're returning planned state, it won't be present unless explicitly set
        }
        _ => panic!("Expected network[0] to be a map"),
    }
}

#[tokio::test]
async fn test_mixed_blocks_and_string_attributes() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1,
              "net0": "virtio,bridge=vmbr0,firewall=1",
              "net1": "e1000,bridge=vmbr1",
              "scsi0": "local-lvm:10,format=raw",
              "ide3": "local:cloudinit"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "net0": "virtio=BC:24:11:AA:BB:CC,bridge=vmbr0,firewall=1",
                    "net1": "e1000=DE:AD:BE:EF:00:01,bridge=vmbr1",
                    "scsi0": "local-lvm:vm-100-disk-0,format=raw,size=10G",
                    "ide3": "local:cloudinit"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();

    let mut net0 = HashMap::new();
    net0.insert("id".to_string(), Dynamic::Number(0.0));
    net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
    net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
    net0.insert("firewall".to_string(), Dynamic::Bool(true));

    config
        .set_list(&AttributePath::new("network"), vec![Dynamic::Map(net0)])
        .unwrap();

    config
        .set_string(
            &AttributePath::new("net1"),
            "e1000,bridge=vmbr1".to_string(),
        )
        .unwrap();

    let mut disk0 = HashMap::new();
    disk0.insert("slot".to_string(), Dynamic::String("scsi0".to_string()));
    disk0.insert("type".to_string(), Dynamic::String("scsi".to_string()));
    disk0.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    disk0.insert("size".to_string(), Dynamic::String("10G".to_string()));
    disk0.insert("format".to_string(), Dynamic::String("raw".to_string()));

    config
        .set_list(&AttributePath::new("disk"), vec![Dynamic::Map(disk0)])
        .unwrap();

    // Use cloudinit_drive block instead of string attribute
    let mut cloudinit = HashMap::new();
    cloudinit.insert("slot".to_string(), Dynamic::String("ide3".to_string()));
    cloudinit.insert("storage".to_string(), Dynamic::String("local".to_string()));
    config
        .set_list(
            &AttributePath::new("cloudinit_drive"),
            vec![Dynamic::Map(cloudinit)],
        )
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    let networks = response
        .new_state
        .get_list(&AttributePath::new("network"))
        .unwrap();
    assert_eq!(networks.len(), 1);

    let net1 = response
        .new_state
        .get_string(&AttributePath::new("net1"))
        .unwrap();
    assert_eq!(net1, "e1000,bridge=vmbr1");

    let disks = response
        .new_state
        .get_list(&AttributePath::new("disk"))
        .unwrap();
    assert_eq!(disks.len(), 1);

    // Verify cloudinit_drive blocks were populated correctly
    let cloudinit_drives = response
        .new_state
        .get_list(&AttributePath::new("cloudinit_drive"))
        .unwrap();
    assert_eq!(cloudinit_drives.len(), 1);
}

#[tokio::test]
async fn test_vm_creation_with_mac_address_specified() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1,
              "net0": "virtio,bridge=vmbr0,firewall=1,macaddr=AA:BB:CC:DD:EE:FF"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "net0": "virtio=AA:BB:CC:DD:EE:FF,bridge=vmbr0,firewall=1"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();

    let mut net0 = HashMap::new();
    net0.insert("id".to_string(), Dynamic::Number(0.0));
    net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
    net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
    net0.insert("firewall".to_string(), Dynamic::Bool(true));
    net0.insert(
        "macaddr".to_string(),
        Dynamic::String("AA:BB:CC:DD:EE:FF".to_string()),
    );

    config
        .set_list(&AttributePath::new("network"), vec![Dynamic::Map(net0)])
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    let networks = response
        .new_state
        .get_list(&AttributePath::new("network"))
        .unwrap();
    assert_eq!(networks.len(), 1);

    match &networks[0] {
        Dynamic::Map(net0_block) => {
            assert_eq!(
                net0_block.get("macaddr").unwrap(),
                &Dynamic::String("AA:BB:CC:DD:EE:FF".to_string())
            );
        }
        _ => panic!("Expected network[0] to be a map"),
    }
}

#[tokio::test]
async fn test_disk_path_transformation() {
    let mut server = Server::new_async().await;
    let _m1 = server
        .mock("POST", "/api2/json/nodes/pve/qemu")
        .match_header("content-type", "application/json")
        .match_body(Matcher::JsonString(
            r#"{
              "vmid": 100,
              "name": "test-vm",
              "memory": 2048,
              "cores": 2,
              "sockets": 1,
              "scsi0": "local-lvm:10,format=raw"
            }"#
            .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
        )
        .create_async()
        .await;

    let _m2 = server
        .mock("GET", "/api2/json/nodes/pve/qemu/100/config")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "vmid": 100,
                    "scsi0": "local-lvm:vm-100-disk-0,format=raw,size=10G"
                }
            }"#,
        )
        .create_async()
        .await;

    let mut resource = QemuVmResource::new();
    let provider_data = create_test_provider_data(&server.url());
    let configure_request = ConfigureResourceRequest {
        provider_data: Some(Arc::new(provider_data) as Arc<dyn Any + Send + Sync>),
    };
    let _ = resource.configure(Context::new(), configure_request).await;

    let mut config = create_test_dynamic_value();

    let mut disk0 = HashMap::new();
    disk0.insert("slot".to_string(), Dynamic::String("scsi0".to_string()));
    disk0.insert("type".to_string(), Dynamic::String("scsi".to_string()));
    disk0.insert(
        "storage".to_string(),
        Dynamic::String("local-lvm".to_string()),
    );
    disk0.insert("size".to_string(), Dynamic::String("10G".to_string()));
    disk0.insert("format".to_string(), Dynamic::String("raw".to_string()));

    config
        .set_list(&AttributePath::new("disk"), vec![Dynamic::Map(disk0)])
        .unwrap();

    let ctx = Context::new();
    let request = CreateResourceRequest {
        type_name: "proxmox_qemu_vm".to_string(),
        config: config.clone(),
        planned_state: config,
        planned_private: vec![],
        provider_meta: Some(DynamicValue::null()),
    };

    let response = resource.create(ctx, request).await;
    if !response.diagnostics.is_empty() {
        for diag in &response.diagnostics {
            eprintln!("Diagnostic: {} - {}", diag.summary, diag.detail);
        }
    }
    assert!(response.diagnostics.is_empty());

    let disks = response
        .new_state
        .get_list(&AttributePath::new("disk"))
        .unwrap();
    assert_eq!(disks.len(), 1);

    match &disks[0] {
        Dynamic::Map(disk0_block) => {
            assert_eq!(
                disk0_block.get("size").unwrap(),
                &Dynamic::String("10G".to_string())
            );
        }
        _ => panic!("Expected disk[0] to be a map"),
    }
}
