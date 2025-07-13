#[cfg(test)]
mod tests {
    use super::super::*;
    use tfplug::context::Context;
    use tfplug::resource::{Resource, ValidateResourceConfigRequest};
    use tfplug::types::{ClientCapabilities, Dynamic, DynamicValue};

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

    #[test]
    fn test_normalize_network_config_sorts_parameters() {
        // Test that parameters are sorted alphabetically
        let net_config = "virtio,bridge=vmbr0,firewall=1,tag=100";
        let normalized = QemuVmResource::normalize_network_config(net_config, None);
        assert_eq!(normalized, "virtio,bridge=vmbr0,firewall=1,tag=100");

        // Test with different order
        let net_config = "virtio,tag=100,bridge=vmbr0,firewall=1";
        let normalized = QemuVmResource::normalize_network_config(net_config, None);
        assert_eq!(normalized, "virtio,bridge=vmbr0,firewall=1,tag=100");

        // Test with MAC address that should be removed
        let net_config = "virtio=BA:88:CB:76:75:D6,tag=100,bridge=vmbr0,firewall=1";
        let normalized = QemuVmResource::normalize_network_config(
            net_config,
            Some("virtio,bridge=vmbr0,tag=100,firewall=1"),
        );
        assert_eq!(normalized, "virtio,bridge=vmbr0,firewall=1,tag=100");

        // Test with MAC address that should be kept
        let net_config = "virtio=BA:88:CB:76:75:D6,tag=100,bridge=vmbr0,firewall=1";
        let normalized = QemuVmResource::normalize_network_config(
            net_config,
            Some("virtio=BA:88:CB:76:75:D6,bridge=vmbr0,tag=100,firewall=1"),
        );
        assert_eq!(
            normalized,
            "virtio=BA:88:CB:76:75:D6,bridge=vmbr0,firewall=1,tag=100"
        );
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

    #[test]
    fn test_resource_factory() {
        let factory: fn() -> Box<dyn ResourceWithConfigure> = || Box::new(QemuVmResource::new());
        let resource = factory();
        assert_eq!(resource.type_name(), "proxmox_qemu_vm");
    }

    #[test]
    fn test_extract_vm_config() {
        let resource = QemuVmResource::new();
        let config = create_test_dynamic_value();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (node, vmid, create_request) = result.unwrap();
        assert_eq!(node, "pve");
        assert_eq!(vmid, 100);
        assert_eq!(create_request.vmid, 100);
        assert_eq!(create_request.name, Some("test-vm".to_string()));
        assert_eq!(create_request.memory, Some(2048));
        assert_eq!(create_request.cores, Some(2));
        assert_eq!(create_request.sockets, Some(1));
    }
}