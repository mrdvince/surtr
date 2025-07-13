use mockito::{Matcher, Server};
use proxmox::api::Client;
use proxmox::resources::nodes::QemuVmResource;
use proxmox::ProxmoxProviderData;
use std::any::Any;
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
    obj.insert("node".to_string(), Dynamic::String("pve".to_string()));
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
    assert!(attrs.iter().any(|a| a.name == "node" && a.required));
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
    updated_obj.insert("node".to_string(), Dynamic::String("pve".to_string()));
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
            .get_string(&AttributePath::new("node"))
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
    assert!(response.diagnostics.is_empty());

    // Verify network interface was populated with normalized value
    let net0 = response
        .new_state
        .get_string(&AttributePath::new("net0"))
        .unwrap();
    assert_eq!(net0, "virtio,bridge=vmbr0,firewall=1");
}