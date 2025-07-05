#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::api::Client;
    use crate::api::nodes::CreateQemuRequest;
    use crate::ProxmoxProviderData;
    use std::sync::Arc;
    use tfplug::context::Context;
    use tfplug::resource::{
        ConfigureResourceRequest, CreateResourceRequest,
        DeleteResourceRequest,
        ImportResourceStateRequest, ReadResourceRequest,
        Resource, ResourceMetadataRequest,
        ResourceSchemaRequest, ResourceWithConfigure, ResourceWithImportState,
        UpdateResourceRequest, ValidateResourceConfigRequest,
    };
    use tfplug::types::{AttributePath, ClientCapabilities, Dynamic, DynamicValue};
    use std::any::Any;
    use mockito::{Matcher, Server};

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

    #[test]
    fn test_resource_type_name() {
        let resource = QemuVmResource::new();
        assert_eq!(resource.type_name(), "proxmox_qemu_vm");
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
    async fn test_validate_valid_config() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config: create_test_dynamic_value(),
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };
        
        let response = resource.validate(ctx, request).await;
        assert!(response.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn test_validate_invalid_vmid_too_low() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        
        let mut obj = std::collections::HashMap::new();
        obj.insert("node".to_string(), Dynamic::String("pve".to_string()));
        obj.insert("vmid".to_string(), Dynamic::Number(50.0)); // Invalid: < 100
        obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
        
        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config: DynamicValue::new(Dynamic::Map(obj)),
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };
        
        let response = resource.validate(ctx, request).await;
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0].summary.contains("Invalid VMID"));
    }

    #[tokio::test]
    async fn test_validate_invalid_cores() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        
        let mut obj = std::collections::HashMap::new();
        obj.insert("node".to_string(), Dynamic::String("pve".to_string()));
        obj.insert("vmid".to_string(), Dynamic::Number(100.0));
        obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
        obj.insert("cores".to_string(), Dynamic::Number(200.0)); // Invalid: > 128
        
        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config: DynamicValue::new(Dynamic::Map(obj)),
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };
        
        let response = resource.validate(ctx, request).await;
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0].summary.contains("Invalid cores"));
    }

    #[tokio::test]
    async fn test_validate_invalid_memory() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        
        let mut obj = std::collections::HashMap::new();
        obj.insert("node".to_string(), Dynamic::String("pve".to_string()));
        obj.insert("vmid".to_string(), Dynamic::Number(100.0));
        obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
        obj.insert("memory".to_string(), Dynamic::Number(10.0)); // Invalid: < 16
        
        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config: DynamicValue::new(Dynamic::Map(obj)),
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };
        
        let response = resource.validate(ctx, request).await;
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0].summary.contains("Invalid memory"));
    }

    #[tokio::test]
    async fn test_validate_invalid_bios() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        
        let mut obj = std::collections::HashMap::new();
        obj.insert("node".to_string(), Dynamic::String("pve".to_string()));
        obj.insert("vmid".to_string(), Dynamic::Number(100.0));
        obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
        obj.insert("bios".to_string(), Dynamic::String("invalid".to_string())); // Invalid
        
        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config: DynamicValue::new(Dynamic::Map(obj)),
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };
        
        let response = resource.validate(ctx, request).await;
        assert_eq!(response.diagnostics.len(), 1);
        assert!(response.diagnostics[0].summary.contains("Invalid BIOS"));
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
        assert!(response.diagnostics[0].summary.contains("Provider not configured"));
    }

    #[tokio::test]
    async fn test_create_successful() {
        let mut server = Server::new_async().await;
        let _m = server.mock("POST", "/api2/json/nodes/pve/qemu")
            .match_header("content-type", "application/x-www-form-urlencoded")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("vmid".into(), "100".into()),
                Matcher::UrlEncoded("name".into(), "test-vm".into()),
                Matcher::UrlEncoded("memory".into(), "2048".into()),
                Matcher::UrlEncoded("cores".into(), "2".into()),
                Matcher::UrlEncoded("sockets".into(), "1".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#)
            .create_async()
            .await;

        let mut resource = QemuVmResource::new();
        resource.provider_data = Some(create_test_provider_data(&server.url()));
        
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
        assert!(response.diagnostics[0].summary.contains("Provider not configured"));
    }

    #[tokio::test]
    async fn test_read_successful() {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/api2/json/nodes/pve/qemu/100/config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "data": {
                    "name": "test-vm-updated",
                    "cores": 4,
                    "memory": 4096,
                    "sockets": 2,
                    "cpu": "host",
                    "ostype": "l26"
                }
            }"#)
            .create_async()
            .await;

        let mut resource = QemuVmResource::new();
        resource.provider_data = Some(create_test_provider_data(&server.url()));
        
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
        assert_eq!(new_state.get_string(&AttributePath::new("name")).unwrap(), "test-vm-updated");
        assert_eq!(new_state.get_number(&AttributePath::new("cores")).unwrap(), 4.0);
        assert_eq!(new_state.get_number(&AttributePath::new("memory")).unwrap(), 4096.0);
        assert_eq!(new_state.get_number(&AttributePath::new("sockets")).unwrap(), 2.0);
    }

    #[tokio::test]
    async fn test_read_vm_not_found() {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/api2/json/nodes/pve/qemu/100/config")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "data": null,
                "errors": {
                    "vmid": "VM 100 not found"
                }
            }"#)
            .create_async()
            .await;

        let mut resource = QemuVmResource::new();
        resource.provider_data = Some(create_test_provider_data(&server.url()));
        
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
    async fn test_update_successful() {
        let mut server = Server::new_async().await;
        let _m = server.mock("POST", "/api2/json/nodes/pve/qemu/100/config")
            .match_header("content-type", "application/x-www-form-urlencoded")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("memory".into(), "4096".into()),
                Matcher::UrlEncoded("cores".into(), "4".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "data": null
            }"#)
            .create_async()
            .await;

        let mut resource = QemuVmResource::new();
        resource.provider_data = Some(create_test_provider_data(&server.url()));
        
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
        let _m = server.mock("DELETE", "/api2/json/nodes/pve/qemu/100")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "data": "UPID:pve:00001234:00000000:5F000000:qmdestroy:100:root@pam:"
            }"#)
            .create_async()
            .await;

        let mut resource = QemuVmResource::new();
        resource.provider_data = Some(create_test_provider_data(&server.url()));
        
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
    async fn test_import_state() {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/api2/json/nodes/pve/qemu/100/config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "data": {
                    "name": "imported-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1
                }
            }"#)
            .create_async()
            .await;

        let mut resource = QemuVmResource::new();
        resource.provider_data = Some(create_test_provider_data(&server.url()));
        
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
        assert_eq!(imported.state.get_string(&AttributePath::new("node")).unwrap(), "pve");
        assert_eq!(imported.state.get_number(&AttributePath::new("vmid")).unwrap(), 100.0);
        assert_eq!(imported.state.get_string(&AttributePath::new("name")).unwrap(), "imported-vm");
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
        assert!(response.diagnostics[0].summary.contains("Invalid import ID"));
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
        assert!(resource.provider_data.is_some());
    }

    #[tokio::test]
    async fn test_extract_vm_config() {
        let resource = QemuVmResource::new();
        let config = create_test_dynamic_value();
        
        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());
        
        let (node, vmid, create_request): (String, u32, CreateQemuRequest) = result.unwrap();
        assert_eq!(node, "pve");
        assert_eq!(vmid, 100);
        assert_eq!(create_request.vmid, 100);
        assert_eq!(create_request.name, Some("test-vm".to_string()));
        assert_eq!(create_request.memory, Some(2048));
        assert_eq!(create_request.cores, Some(2));
        assert_eq!(create_request.sockets, Some(1));
    }

    #[test]
    fn test_resource_factory() {
        let factory: fn() -> Box<dyn ResourceWithConfigure> = || Box::new(QemuVmResource::new());
        let resource = factory();
        assert_eq!(resource.type_name(), "proxmox_qemu_vm");
    }
}