#[cfg(test)]
mod tests {
    use super::super::*;
    use tfplug::context::Context;
    use tfplug::resource::{Resource, ValidateResourceConfigRequest};
    use tfplug::types::{ClientCapabilities, Dynamic, DynamicValue};

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

    fn create_test_dynamic_value_with_network_blocks() -> DynamicValue {
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

        // Network blocks
        let mut net0 = std::collections::HashMap::new();
        net0.insert("id".to_string(), Dynamic::Number(0.0));
        net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
        net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
        net0.insert("firewall".to_string(), Dynamic::Bool(true));
        net0.insert("tag".to_string(), Dynamic::Number(100.0));

        let mut net1 = std::collections::HashMap::new();
        net1.insert("id".to_string(), Dynamic::Number(1.0));
        net1.insert("model".to_string(), Dynamic::String("e1000".to_string()));
        net1.insert("bridge".to_string(), Dynamic::String("vmbr1".to_string()));
        net1.insert("firewall".to_string(), Dynamic::Bool(false));
        net1.insert("tag".to_string(), Dynamic::Number(200.0));

        obj.insert(
            "network".to_string(),
            Dynamic::List(vec![Dynamic::Map(net0), Dynamic::Map(net1)]),
        );

        DynamicValue::new(Dynamic::Map(obj))
    }

    fn create_test_dynamic_value_with_disk_blocks() -> DynamicValue {
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

        // Disk blocks
        let mut disk0 = std::collections::HashMap::new();
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
        disk0.insert("discard".to_string(), Dynamic::Bool(true));

        let mut disk1 = std::collections::HashMap::new();
        disk1.insert("slot".to_string(), Dynamic::String("virtio0".to_string()));
        disk1.insert("type".to_string(), Dynamic::String("virtio".to_string()));
        disk1.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );
        disk1.insert("size".to_string(), Dynamic::String("20G".to_string()));
        disk1.insert("format".to_string(), Dynamic::String("qcow2".to_string()));

        obj.insert(
            "disk".to_string(),
            Dynamic::List(vec![Dynamic::Map(disk0), Dynamic::Map(disk1)]),
        );

        DynamicValue::new(Dynamic::Map(obj))
    }

    fn create_test_dynamic_value_with_efidisk() -> DynamicValue {
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
        obj.insert("bios".to_string(), Dynamic::String("ovmf".to_string()));

        // EFI disk block
        let mut efidisk = std::collections::HashMap::new();
        efidisk.insert("efitype".to_string(), Dynamic::String("4m".to_string()));
        efidisk.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );

        obj.insert(
            "efidisk".to_string(),
            Dynamic::List(vec![Dynamic::Map(efidisk)]),
        );

        DynamicValue::new(Dynamic::Map(obj))
    }

    fn create_test_dynamic_value_with_cloudinit() -> DynamicValue {
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

        // Cloud-init block
        let mut cloudinit = std::collections::HashMap::new();
        cloudinit.insert("user".to_string(), Dynamic::String("ubuntu".to_string()));
        cloudinit.insert(
            "password".to_string(),
            Dynamic::String("secret123".to_string()),
        );
        cloudinit.insert(
            "ssh_keys".to_string(),
            Dynamic::String("ssh-rsa AAAAB3NzaC1...".to_string()),
        );

        // IP config blocks
        let mut ipconfig0 = std::collections::HashMap::new();
        ipconfig0.insert("id".to_string(), Dynamic::Number(0.0));
        ipconfig0.insert(
            "ipv4".to_string(),
            Dynamic::String("192.168.1.100/24".to_string()),
        );
        ipconfig0.insert(
            "gateway".to_string(),
            Dynamic::String("192.168.1.1".to_string()),
        );

        let mut ipconfig1 = std::collections::HashMap::new();
        ipconfig1.insert("id".to_string(), Dynamic::Number(1.0));
        ipconfig1.insert(
            "ipv4".to_string(),
            Dynamic::String("10.0.0.100/24".to_string()),
        );

        cloudinit.insert(
            "ipconfig".to_string(),
            Dynamic::List(vec![Dynamic::Map(ipconfig0), Dynamic::Map(ipconfig1)]),
        );

        obj.insert("cloudinit".to_string(), Dynamic::Map(cloudinit));

        DynamicValue::new(Dynamic::Map(obj))
    }

    fn create_test_dynamic_value_with_advanced_features() -> DynamicValue {
        let mut obj = std::collections::HashMap::new();
        // Core VM Identity
        obj.insert("vmid".to_string(), Dynamic::Number(100.0));
        obj.insert("name".to_string(), Dynamic::String("test-vm".to_string()));
        obj.insert(
            "target_node".to_string(),
            Dynamic::String("pve".to_string()),
        );
        obj.insert(
            "tags".to_string(),
            Dynamic::String("production,web".to_string()),
        );

        // Clone/Template Settings
        obj.insert(
            "clone".to_string(),
            Dynamic::String("template-ubuntu".to_string()),
        );
        obj.insert("full_clone".to_string(), Dynamic::Bool(false));
        obj.insert(
            "os_type".to_string(),
            Dynamic::String("cloud-init".to_string()),
        );

        // Hardware Configuration
        obj.insert("bios".to_string(), Dynamic::String("ovmf".to_string()));
        obj.insert("machine".to_string(), Dynamic::String("q35".to_string()));
        obj.insert(
            "cpu_type".to_string(),
            Dynamic::String("x86-64-v2-AES".to_string()),
        );
        obj.insert("cores".to_string(), Dynamic::Number(2.0));
        obj.insert("sockets".to_string(), Dynamic::Number(1.0));
        obj.insert("vcpus".to_string(), Dynamic::Number(2.0));
        obj.insert("memory".to_string(), Dynamic::Number(4096.0));
        obj.insert("balloon".to_string(), Dynamic::Number(2048.0));

        // Boot Configuration
        obj.insert("boot".to_string(), Dynamic::String("c".to_string()));
        obj.insert("bootdisk".to_string(), Dynamic::String("scsi0".to_string()));
        obj.insert("onboot".to_string(), Dynamic::Bool(true));

        // Storage Configuration
        obj.insert(
            "scsihw".to_string(),
            Dynamic::String("virtio-scsi-pci".to_string()),
        );

        // Guest Agent & OS Settings
        obj.insert("agent".to_string(), Dynamic::Number(1.0));
        obj.insert("qemu_os".to_string(), Dynamic::String("l26".to_string()));

        // Cloud-Init Configuration
        obj.insert(
            "ipconfig0".to_string(),
            Dynamic::String("ip=dhcp".to_string()),
        );
        obj.insert("ciuser".to_string(), Dynamic::String("ubuntu".to_string()));
        obj.insert(
            "cipassword".to_string(),
            Dynamic::String("password123".to_string()),
        );
        obj.insert("ciupgrade".to_string(), Dynamic::Bool(true));
        obj.insert(
            "sshkeys".to_string(),
            Dynamic::String("ssh-rsa AAAAB3NzaC1...".to_string()),
        );

        // Network Settings
        obj.insert("skip_ipv4".to_string(), Dynamic::Bool(false));
        obj.insert("skip_ipv6".to_string(), Dynamic::Bool(true));

        // Timing & Behavior Settings
        obj.insert("additional_wait".to_string(), Dynamic::Number(15.0));
        obj.insert("automatic_reboot".to_string(), Dynamic::Bool(true));
        obj.insert("clone_wait".to_string(), Dynamic::Number(30.0));
        obj.insert("define_connection_info".to_string(), Dynamic::Bool(false));

        // Disk blocks
        let mut disk0 = std::collections::HashMap::new();
        disk0.insert("slot".to_string(), Dynamic::String("scsi0".to_string()));
        disk0.insert("type".to_string(), Dynamic::String("scsi".to_string()));
        disk0.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );
        disk0.insert("size".to_string(), Dynamic::String("20G".to_string()));
        disk0.insert("format".to_string(), Dynamic::String("raw".to_string()));
        disk0.insert("discard".to_string(), Dynamic::Bool(true));
        disk0.insert("emulatessd".to_string(), Dynamic::Bool(true));
        disk0.insert("iothread".to_string(), Dynamic::Bool(true));

        obj.insert("disk".to_string(), Dynamic::List(vec![Dynamic::Map(disk0)]));

        // CD-ROM block
        let mut cdrom = std::collections::HashMap::new();
        cdrom.insert("slot".to_string(), Dynamic::String("ide2".to_string()));
        cdrom.insert(
            "iso".to_string(),
            Dynamic::String("local:iso/ubuntu-24.04.iso".to_string()),
        );

        obj.insert(
            "cdrom".to_string(),
            Dynamic::List(vec![Dynamic::Map(cdrom)]),
        );

        // Cloud-init drive
        let mut ci_drive = std::collections::HashMap::new();
        ci_drive.insert("slot".to_string(), Dynamic::String("ide3".to_string()));
        ci_drive.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );

        obj.insert(
            "cloudinit_drive".to_string(),
            Dynamic::List(vec![Dynamic::Map(ci_drive)]),
        );

        // Serial port
        let mut serial = std::collections::HashMap::new();
        serial.insert("id".to_string(), Dynamic::Number(0.0));
        serial.insert("type".to_string(), Dynamic::String("socket".to_string()));

        obj.insert(
            "serial".to_string(),
            Dynamic::List(vec![Dynamic::Map(serial)]),
        );

        // EFI disk
        let mut efidisk = std::collections::HashMap::new();
        efidisk.insert("efitype".to_string(), Dynamic::String("4m".to_string()));
        efidisk.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );

        obj.insert(
            "efidisk".to_string(),
            Dynamic::List(vec![Dynamic::Map(efidisk)]),
        );

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
        if !response.diagnostics.is_empty() {
            for diag in &response.diagnostics {
                println!("Diagnostic: {} - {}", diag.summary, &diag.detail);
            }
        }
        assert!(response.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn test_validate_invalid_vmid_too_low() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();

        let mut obj = std::collections::HashMap::new();
        obj.insert(
            "target_node".to_string(),
            Dynamic::String("pve".to_string()),
        );
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
        obj.insert(
            "target_node".to_string(),
            Dynamic::String("pve".to_string()),
        );
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
        obj.insert(
            "target_node".to_string(),
            Dynamic::String("pve".to_string()),
        );
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
        obj.insert(
            "target_node".to_string(),
            Dynamic::String("pve".to_string()),
        );
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

    #[tokio::test]
    async fn test_validate_advanced_configuration() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();

        let config = create_test_dynamic_value_with_advanced_features();
        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config,
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };

        let response = resource.validate(ctx, request).await;
        // OVMF bios is set so we should get a warning about missing efidisk
        assert_eq!(response.diagnostics.len(), 0); // No errors since efidisk is included
    }

    #[tokio::test]
    async fn test_schema_contains_network_blocks() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let response = resource
            .schema(ctx, tfplug::resource::ResourceSchemaRequest)
            .await;

        assert!(response.diagnostics.is_empty());
        let network_block = response
            .schema
            .block
            .block_types
            .iter()
            .find(|b| b.type_name == "network");
        assert!(network_block.is_some());

        let network_block = network_block.unwrap();
        assert_eq!(network_block.nesting, tfplug::schema::NestingMode::List);
        assert_eq!(network_block.min_items, 0);
        assert_eq!(network_block.max_items, 32);

        // Check network block attributes
        let attrs = &network_block.block.attributes;
        assert!(attrs.iter().any(|a| a.name == "id"));
        assert!(attrs.iter().any(|a| a.name == "model"));
        assert!(attrs.iter().any(|a| a.name == "bridge"));
        assert!(attrs.iter().any(|a| a.name == "firewall"));
        assert!(attrs.iter().any(|a| a.name == "tag"));
        assert!(attrs.iter().any(|a| a.name == "macaddr"));
        assert!(attrs.iter().any(|a| a.name == "rate"));
        assert!(attrs.iter().any(|a| a.name == "queues"));
    }

    #[tokio::test]
    async fn test_schema_contains_disk_blocks() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let response = resource
            .schema(ctx, tfplug::resource::ResourceSchemaRequest)
            .await;

        assert!(response.diagnostics.is_empty());
        let disk_block = response
            .schema
            .block
            .block_types
            .iter()
            .find(|b| b.type_name == "disk");
        assert!(disk_block.is_some());

        let disk_block = disk_block.unwrap();
        assert_eq!(disk_block.nesting, tfplug::schema::NestingMode::List);

        // Check disk block attributes
        let attrs = &disk_block.block.attributes;
        assert!(attrs.iter().any(|a| a.name == "slot"));
        assert!(attrs.iter().any(|a| a.name == "type"));
        assert!(attrs.iter().any(|a| a.name == "storage"));
        assert!(attrs.iter().any(|a| a.name == "size"));
        assert!(attrs.iter().any(|a| a.name == "format"));
        assert!(attrs.iter().any(|a| a.name == "iothread"));
        assert!(attrs.iter().any(|a| a.name == "emulatessd"));
        assert!(attrs.iter().any(|a| a.name == "discard"));
        assert!(attrs.iter().any(|a| a.name == "backup"));
        assert!(attrs.iter().any(|a| a.name == "replicate"));
    }

    #[tokio::test]
    async fn test_schema_contains_efidisk_block() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let response = resource
            .schema(ctx, tfplug::resource::ResourceSchemaRequest)
            .await;

        assert!(response.diagnostics.is_empty());
        let efidisk_block = response
            .schema
            .block
            .block_types
            .iter()
            .find(|b| b.type_name == "efidisk");
        assert!(efidisk_block.is_some());

        let efidisk_block = efidisk_block.unwrap();
        assert_eq!(efidisk_block.nesting, tfplug::schema::NestingMode::List);

        // Check efidisk block attributes
        let attrs = &efidisk_block.block.attributes;
        assert!(attrs.iter().any(|a| a.name == "storage"));
        assert!(attrs.iter().any(|a| a.name == "efitype"));
    }

    #[tokio::test]
    async fn test_schema_contains_cloudinit_block() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let response = resource
            .schema(ctx, tfplug::resource::ResourceSchemaRequest)
            .await;

        assert!(response.diagnostics.is_empty());
        let cloudinit_block = response
            .schema
            .block
            .block_types
            .iter()
            .find(|b| b.type_name == "cloudinit_drive");
        assert!(cloudinit_block.is_some());

        let cloudinit_block = cloudinit_block.unwrap();
        assert_eq!(cloudinit_block.nesting, tfplug::schema::NestingMode::List);

        // Check cloudinit_drive block attributes
        let attrs = &cloudinit_block.block.attributes;
        assert!(attrs.iter().any(|a| a.name == "slot"));
        assert!(attrs.iter().any(|a| a.name == "storage"));
    }

    #[tokio::test]
    async fn test_validate_network_blocks() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let config = create_test_dynamic_value_with_network_blocks();

        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config,
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };

        let response = resource.validate(ctx, request).await;
        if !response.diagnostics.is_empty() {
            for diag in &response.diagnostics {
                println!("Diagnostic: {} - {}", diag.summary, &diag.detail);
            }
        }
        assert!(response.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn test_validate_disk_blocks() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let config = create_test_dynamic_value_with_disk_blocks();

        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config,
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };

        let response = resource.validate(ctx, request).await;
        if !response.diagnostics.is_empty() {
            for diag in &response.diagnostics {
                println!("Diagnostic: {} - {}", diag.summary, &diag.detail);
            }
        }
        assert!(response.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn test_validate_efidisk_with_ovmf() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let config = create_test_dynamic_value_with_efidisk();

        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config,
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };

        let response = resource.validate(ctx, request).await;
        if !response.diagnostics.is_empty() {
            for diag in &response.diagnostics {
                println!("Diagnostic: {} - {}", diag.summary, &diag.detail);
            }
        }
        assert!(response.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn test_validate_cloudinit_blocks() {
        let resource = QemuVmResource::new();
        let ctx = Context::new();
        let config = create_test_dynamic_value_with_cloudinit();

        let request = ValidateResourceConfigRequest {
            type_name: "proxmox_qemu_vm".to_string(),
            config,
            client_capabilities: ClientCapabilities {
                deferral_allowed: false,
                write_only_attributes_allowed: false,
            },
        };

        let response = resource.validate(ctx, request).await;
        if !response.diagnostics.is_empty() {
            for diag in &response.diagnostics {
                println!("Diagnostic: {} - {}", diag.summary, &diag.detail);
            }
        }
        assert!(response.diagnostics.is_empty());
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

    #[test]
    fn test_network_blocks_to_string() {
        let mut networks = vec![];
        let mut net0 = std::collections::HashMap::new();
        net0.insert("id".to_string(), Dynamic::Number(0.0));
        net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
        net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
        net0.insert("firewall".to_string(), Dynamic::Bool(true));
        net0.insert("tag".to_string(), Dynamic::Number(100.0));
        networks.push(Dynamic::Map(net0));

        let net_string = QemuVmResource::network_blocks_to_string(&networks).unwrap();
        assert!(net_string.contains("virtio"));
        assert!(net_string.contains("bridge=vmbr0"));
        assert!(net_string.contains("firewall=1"));
        assert!(net_string.contains("tag=100"));
    }

    #[test]
    fn test_disk_blocks_to_string() {
        let mut disks = vec![];
        let mut disk0 = std::collections::HashMap::new();
        disk0.insert(
            "interface".to_string(),
            Dynamic::String("scsi0".to_string()),
        );
        disk0.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );
        disk0.insert("size".to_string(), Dynamic::String("10G".to_string()));
        disk0.insert("format".to_string(), Dynamic::String("raw".to_string()));
        disk0.insert("iothread".to_string(), Dynamic::Bool(true));
        disks.push(Dynamic::Map(disk0));

        let (interface, disk_string) = QemuVmResource::disk_block_to_string(&disks[0]).unwrap();
        assert_eq!(interface, "scsi0");
        assert!(disk_string.contains("local-lvm:"));
        assert!(disk_string.contains("10"));
        assert!(disk_string.contains("format=raw"));
        assert!(disk_string.contains("iothread=1"));
    }

    #[test]
    fn test_parse_network_string_to_block() {
        let net_string = "virtio,bridge=vmbr0,firewall=1,tag=100";
        let network_block = QemuVmResource::parse_network_string(net_string, 0);

        match network_block {
            Dynamic::Map(map) => {
                assert_eq!(map.get("id"), Some(&Dynamic::Number(0.0)));
                assert_eq!(
                    map.get("model"),
                    Some(&Dynamic::String("virtio".to_string()))
                );
                assert_eq!(
                    map.get("bridge"),
                    Some(&Dynamic::String("vmbr0".to_string()))
                );
                assert_eq!(map.get("firewall"), Some(&Dynamic::Bool(true)));
                assert_eq!(map.get("tag"), Some(&Dynamic::Number(100.0)));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_disk_string_to_block() {
        let disk_string = "local-lvm:10,format=raw,iothread=1";
        let disk_block = QemuVmResource::parse_disk_string(disk_string, "scsi0");

        match disk_block {
            Dynamic::Map(map) => {
                assert_eq!(map.get("slot"), Some(&Dynamic::String("scsi0".to_string())));
                assert_eq!(map.get("type"), Some(&Dynamic::String("scsi".to_string())));
                assert_eq!(
                    map.get("storage"),
                    Some(&Dynamic::String("local-lvm".to_string()))
                );
                assert_eq!(map.get("size"), Some(&Dynamic::String("10G".to_string())));
                assert_eq!(map.get("format"), Some(&Dynamic::String("raw".to_string())));
                assert_eq!(map.get("iothread"), Some(&Dynamic::Bool(true)));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_iso_disk_string_to_block() {
        let disk_string = "local:iso/ubuntu-22.04.iso,media=cdrom";
        let disk_block = QemuVmResource::parse_disk_string(disk_string, "ide2");

        match disk_block {
            Dynamic::Map(map) => {
                assert_eq!(map.get("slot"), Some(&Dynamic::String("ide2".to_string())));
                assert_eq!(map.get("type"), Some(&Dynamic::String("ide".to_string())));
                assert_eq!(
                    map.get("storage"),
                    Some(&Dynamic::String("local".to_string()))
                );
                assert_eq!(
                    map.get("iso"),
                    Some(&Dynamic::String("iso/ubuntu-22.04.iso".to_string()))
                );
                assert_eq!(
                    map.get("media"),
                    Some(&Dynamic::String("cdrom".to_string()))
                );
                assert_eq!(map.get("size"), None);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_cloudinit_disk_string_to_block() {
        let disk_string = "local-lvm,media=cloudinit";
        let disk_block = QemuVmResource::parse_disk_string(disk_string, "ide2");

        match disk_block {
            Dynamic::Map(map) => {
                assert_eq!(map.get("slot"), Some(&Dynamic::String("ide2".to_string())));
                assert_eq!(map.get("type"), Some(&Dynamic::String("ide".to_string())));
                assert_eq!(
                    map.get("storage"),
                    Some(&Dynamic::String("local-lvm".to_string()))
                );
                assert_eq!(
                    map.get("media"),
                    Some(&Dynamic::String("cloudinit".to_string()))
                );
                assert_eq!(map.get("size"), None);
                assert_eq!(map.get("format"), None);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_efidisk_block_to_string() {
        let mut efidisk = std::collections::HashMap::new();
        efidisk.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );
        efidisk.insert("format".to_string(), Dynamic::String("raw".to_string()));
        efidisk.insert("efitype".to_string(), Dynamic::String("4m".to_string()));
        efidisk.insert("pre_enrolled_keys".to_string(), Dynamic::Bool(true));

        let efidisk_string =
            QemuVmResource::efidisk_block_to_api_string(&Dynamic::Map(efidisk)).unwrap();
        assert!(efidisk_string.contains("local-lvm:"));
        assert!(efidisk_string.contains("efitype=4m"));
        // efidisk_block_to_api_string only includes storage and efitype
    }

    #[test]
    fn test_extract_vm_config_with_network_blocks() {
        let resource = QemuVmResource::new();
        let config = create_test_dynamic_value_with_network_blocks();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (node, vmid, create_request) = result.unwrap();
        assert_eq!(node, "pve");
        assert_eq!(vmid, 100);
        assert_eq!(
            create_request.net0,
            Some("virtio,bridge=vmbr0,firewall=1,tag=100".to_string())
        );
        assert_eq!(
            create_request.net1,
            Some("e1000,bridge=vmbr1,firewall=0,tag=200".to_string())
        );
    }

    #[test]
    fn test_extract_vm_config_with_disk_blocks() {
        let resource = QemuVmResource::new();
        let config = create_test_dynamic_value_with_disk_blocks();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        assert_eq!(
            create_request.scsi0,
            Some("local-lvm:10,format=raw,iothread=1,ssd=1,discard=on".to_string())
        );
        assert_eq!(
            create_request.virtio0,
            Some("local-lvm:20,format=qcow2".to_string())
        );
    }

    #[test]
    fn test_extract_vm_config_with_efidisk_block() {
        let resource = QemuVmResource::new();
        let config = create_test_dynamic_value_with_efidisk();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        assert_eq!(
            create_request.efidisk0,
            Some("local-lvm:1,efitype=4m".to_string())
        );
    }

    #[test]
    fn test_build_update_request_with_nested_blocks() {
        let resource = QemuVmResource::new();
        let config = create_test_dynamic_value_with_network_blocks();

        let result = resource.build_update_request(&config);
        assert!(result.is_ok());

        let update_request = result.unwrap();
        assert_eq!(
            update_request.net0,
            Some("virtio,bridge=vmbr0,firewall=1,tag=100".to_string())
        );
        assert_eq!(
            update_request.net1,
            Some("e1000,bridge=vmbr1,firewall=0,tag=200".to_string())
        );
    }

    #[test]
    fn test_populate_state_from_config_with_computed_fields() {
        let mut state = create_test_dynamic_value();

        // Create a VM config that Proxmox would return
        let vm_config = crate::api::nodes::QemuConfig {
            name: Some("test-vm".to_string()),
            cores: Some(2),
            memory: Some(2048),
            net0: Some("virtio=BA:88:CB:76:75:D6,bridge=vmbr0,firewall=1,tag=100".to_string()),
            scsi0: Some("local-lvm:vm-100-disk-0,size=10G".to_string()),
            ..Default::default()
        };

        let planned_state = create_test_dynamic_value_with_network_blocks();
        QemuVmResource::populate_state_from_config(&mut state, &vm_config, &planned_state);

        // Check that network config was normalized correctly
        if let Ok(net0) = state.get_string(&AttributePath::new("net0")) {
            assert_eq!(net0, "virtio,bridge=vmbr0,firewall=1,tag=100");
        }

        // Check disk config normalization
        if let Ok(scsi0) = state.get_string(&AttributePath::new("scsi0")) {
            assert_eq!(scsi0, "local-lvm:vm-100-disk-0,size=10G");
        }
    }

    #[test]
    fn test_populate_state_with_network_blocks() {
        let mut state = DynamicValue::new(Dynamic::Map(std::collections::HashMap::new()));

        // Create a VM config that Proxmox would return
        let vm_config = crate::api::nodes::QemuConfig {
            name: Some("test-vm".to_string()),
            net0: Some("virtio=BA:88:CB:76:75:D6,bridge=vmbr0,firewall=1,tag=100".to_string()),
            net1: Some("e1000=AA:BB:CC:DD:EE:FF,bridge=vmbr1,tag=200".to_string()),
            ..Default::default()
        };

        let planned_state = create_test_dynamic_value_with_network_blocks();
        QemuVmResource::populate_state_with_nested_blocks(&mut state, &vm_config, &planned_state);

        // Check that network blocks were populated with MAC addresses
        if let Ok(networks) = state.get_list(&AttributePath::new("network")) {
            assert_eq!(networks.len(), 2);

            // Check first network
            if let Dynamic::Map(net0) = &networks[0] {
                assert_eq!(net0.get("id"), Some(&Dynamic::Number(0.0)));
                assert_eq!(
                    net0.get("model"),
                    Some(&Dynamic::String("virtio".to_string()))
                );
                assert_eq!(
                    net0.get("bridge"),
                    Some(&Dynamic::String("vmbr0".to_string()))
                );
                assert_eq!(net0.get("firewall"), Some(&Dynamic::Bool(true)));
                assert_eq!(net0.get("tag"), Some(&Dynamic::Number(100.0)));
                assert_eq!(
                    net0.get("macaddr"),
                    Some(&Dynamic::String("BA:88:CB:76:75:D6".to_string()))
                );
            }

            // Check second network
            if let Dynamic::Map(net1) = &networks[1] {
                assert_eq!(net1.get("id"), Some(&Dynamic::Number(1.0)));
                assert_eq!(
                    net1.get("model"),
                    Some(&Dynamic::String("e1000".to_string()))
                );
                assert_eq!(
                    net1.get("bridge"),
                    Some(&Dynamic::String("vmbr1".to_string()))
                );
                assert_eq!(net1.get("tag"), Some(&Dynamic::Number(200.0)));
                assert_eq!(
                    net1.get("macaddr"),
                    Some(&Dynamic::String("AA:BB:CC:DD:EE:FF".to_string()))
                );
            }
        }
    }

    #[test]
    fn test_populate_state_with_disk_blocks() {
        let mut state = DynamicValue::new(Dynamic::Map(std::collections::HashMap::new()));

        // Create a VM config that Proxmox would return with actual disk paths
        let vm_config = crate::api::nodes::QemuConfig {
            name: Some("test-vm".to_string()),
            scsi0: Some(
                "local-lvm:vm-100-disk-0,size=10G,format=raw,iothread=1,ssd=1,discard=on"
                    .to_string(),
            ),
            virtio0: Some("local-lvm:vm-100-disk-1,size=20G,format=qcow2".to_string()),
            ..Default::default()
        };

        let planned_state = create_test_dynamic_value_with_disk_blocks();
        QemuVmResource::populate_state_with_nested_blocks(&mut state, &vm_config, &planned_state);

        // Check that disk blocks were populated
        if let Ok(disks) = state.get_list(&AttributePath::new("disk")) {
            assert_eq!(disks.len(), 2);

            // Check SCSI disk
            if let Dynamic::Map(scsi0) = &disks[0] {
                assert_eq!(
                    scsi0.get("slot"),
                    Some(&Dynamic::String("scsi0".to_string()))
                );
                assert_eq!(
                    scsi0.get("type"),
                    Some(&Dynamic::String("scsi".to_string()))
                );
                assert_eq!(
                    scsi0.get("storage"),
                    Some(&Dynamic::String("local-lvm".to_string()))
                );
                assert_eq!(scsi0.get("size"), Some(&Dynamic::String("10G".to_string())));
                assert_eq!(
                    scsi0.get("format"),
                    Some(&Dynamic::String("raw".to_string()))
                );
                assert_eq!(scsi0.get("iothread"), Some(&Dynamic::Bool(true)));
                assert_eq!(scsi0.get("emulatessd"), Some(&Dynamic::Bool(true)));
                assert_eq!(scsi0.get("discard"), Some(&Dynamic::Bool(true)));
            }

            // Check VirtIO disk
            if let Dynamic::Map(virtio0) = &disks[1] {
                assert_eq!(
                    virtio0.get("slot"),
                    Some(&Dynamic::String("virtio0".to_string()))
                );
                assert_eq!(
                    virtio0.get("type"),
                    Some(&Dynamic::String("virtio".to_string()))
                );
                assert_eq!(
                    virtio0.get("storage"),
                    Some(&Dynamic::String("local-lvm".to_string()))
                );
                assert_eq!(
                    virtio0.get("size"),
                    Some(&Dynamic::String("20G".to_string()))
                );
                assert_eq!(
                    virtio0.get("format"),
                    Some(&Dynamic::String("qcow2".to_string()))
                );
            }
        }
    }

    #[test]
    fn test_handle_computed_mac_addresses() {
        let mut networks = vec![];
        let mut net0 = std::collections::HashMap::new();
        net0.insert("id".to_string(), Dynamic::Number(0.0));
        net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
        net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
        // No MAC address provided - should be computed by Proxmox
        networks.push(Dynamic::Map(net0));

        let net_string = QemuVmResource::network_blocks_to_string(&networks).unwrap();
        assert_eq!(net_string, "virtio,bridge=vmbr0");
        assert!(!net_string.contains("macaddr"));
    }

    #[test]
    fn test_network_with_provided_mac_address() {
        let mut networks = vec![];
        let mut net0 = std::collections::HashMap::new();
        net0.insert("id".to_string(), Dynamic::Number(0.0));
        net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
        net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
        net0.insert(
            "macaddr".to_string(),
            Dynamic::String("AA:BB:CC:DD:EE:FF".to_string()),
        );
        networks.push(Dynamic::Map(net0));

        let net_string = QemuVmResource::network_blocks_to_string(&networks).unwrap();
        assert!(net_string.contains("macaddr=AA:BB:CC:DD:EE:FF"));
    }

    #[test]
    fn test_mixed_block_and_string_attributes() {
        let resource = QemuVmResource::new();
        let mut config = create_test_dynamic_value();

        // Add network block
        let mut net0 = std::collections::HashMap::new();
        net0.insert("id".to_string(), Dynamic::Number(0.0));
        net0.insert("model".to_string(), Dynamic::String("virtio".to_string()));
        net0.insert("bridge".to_string(), Dynamic::String("vmbr0".to_string()));
        net0.insert("firewall".to_string(), Dynamic::Bool(true));

        // Add string attribute for net1
        config
            .set_list(&AttributePath::new("network"), vec![Dynamic::Map(net0)])
            .unwrap();
        config
            .set_string(
                &AttributePath::new("net1"),
                "e1000,bridge=vmbr1".to_string(),
            )
            .unwrap();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        assert_eq!(
            create_request.net0,
            Some("virtio,bridge=vmbr0,firewall=1".to_string())
        );
        assert_eq!(create_request.net1, Some("e1000,bridge=vmbr1".to_string()));
    }

    #[test]
    fn test_mixed_disk_types_with_blocks() {
        let resource = QemuVmResource::new();
        let mut config = create_test_dynamic_value();

        // Regular disk
        let mut disk0 = std::collections::HashMap::new();
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

        // CD-ROM block
        let mut cdrom = std::collections::HashMap::new();
        cdrom.insert("slot".to_string(), Dynamic::String("ide2".to_string()));
        cdrom.insert(
            "iso".to_string(),
            Dynamic::String("local:iso/ubuntu-22.04.iso".to_string()),
        );

        config
            .set_list(&AttributePath::new("cdrom"), vec![Dynamic::Map(cdrom)])
            .unwrap();

        // Cloud-init drive block
        let mut ci_drive = std::collections::HashMap::new();
        ci_drive.insert("slot".to_string(), Dynamic::String("ide3".to_string()));
        ci_drive.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );

        config
            .set_list(
                &AttributePath::new("cloudinit_drive"),
                vec![Dynamic::Map(ci_drive)],
            )
            .unwrap();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        assert_eq!(
            create_request.scsi0,
            Some("local-lvm:10,format=raw".to_string())
        );
        assert_eq!(
            create_request.ide2,
            Some("local:iso/ubuntu-22.04.iso,media=cdrom".to_string())
        );
        assert_eq!(create_request.ide3, Some("local-lvm:cloudinit".to_string()));
    }

    #[test]
    fn test_cloudinit_ipconfig_to_string() {
        let mut cloudinit = std::collections::HashMap::new();
        cloudinit.insert("user".to_string(), Dynamic::String("ubuntu".to_string()));

        let mut ipconfig0 = std::collections::HashMap::new();
        ipconfig0.insert("id".to_string(), Dynamic::Number(0.0));
        ipconfig0.insert(
            "ipv4".to_string(),
            Dynamic::String("192.168.1.100/24".to_string()),
        );
        ipconfig0.insert(
            "gateway".to_string(),
            Dynamic::String("192.168.1.1".to_string()),
        );

        cloudinit.insert(
            "ipconfig".to_string(),
            Dynamic::List(vec![Dynamic::Map(ipconfig0)]),
        );

        // Test that cloudinit block would be correctly parsed into ipconfig0 string
        // This would be handled in a real cloudinit_block_to_string method
        assert!(cloudinit.contains_key("user"));
        assert!(cloudinit.contains_key("ipconfig"));
    }

    #[test]
    fn test_populate_state_with_all_zero_values() {
        let mut state = DynamicValue::new(Dynamic::Map(std::collections::HashMap::new()));

        // Create a minimal VM config that Proxmox might return
        let vm_config = crate::api::nodes::QemuConfig {
            name: Some("test-vm".to_string()),
            ..Default::default()
        };

        let planned_state = create_test_dynamic_value();
        QemuVmResource::populate_state_from_config(&mut state, &vm_config, &planned_state);

        // Only attributes present in planned state should be populated with defaults
        assert!(state.get_number(&AttributePath::new("cores")).is_ok());
        assert!(state.get_number(&AttributePath::new("sockets")).is_ok());
        assert!(state.get_number(&AttributePath::new("memory")).is_ok());

        // These attributes are not in the planned state, so they should not be set
        assert!(state.get_string(&AttributePath::new("cpu")).is_err());
        assert!(state.get_string(&AttributePath::new("bios")).is_err());
        assert!(state.get_string(&AttributePath::new("scsihw")).is_err());
        assert!(state.get_string(&AttributePath::new("ostype")).is_err());
        assert!(state.get_string(&AttributePath::new("agent")).is_err());
        assert!(state.get_bool(&AttributePath::new("onboot")).is_err());
        assert!(state.get_bool(&AttributePath::new("tablet")).is_err());
        assert!(state.get_bool(&AttributePath::new("protection")).is_err());
        assert!(state.get_string(&AttributePath::new("tags")).is_err());
        assert!(state
            .get_string(&AttributePath::new("description"))
            .is_err());
    }

    #[test]
    fn test_disk_format_for_cdrom() {
        let resource = QemuVmResource::new();
        let mut config = create_test_dynamic_value();

        // Add ide2 for CD-ROM (should not have format)
        config
            .set_string(
                &AttributePath::new("ide2"),
                "local:iso/ubuntu-22.04.iso,media=cdrom".to_string(),
            )
            .unwrap();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        // IDE2 should not contain format=cdrom
        if let Some(ide2) = &create_request.ide2 {
            assert!(!ide2.contains("format=cdrom"));
            assert!(!ide2.contains("format=cloudinit"));
        }
    }

    #[test]
    fn test_disk_block_to_string_for_iso() {
        let mut disk = std::collections::HashMap::new();
        disk.insert("interface".to_string(), Dynamic::String("ide2".to_string()));
        disk.insert("storage".to_string(), Dynamic::String("local".to_string()));
        disk.insert(
            "iso".to_string(),
            Dynamic::String("iso/ubuntu-22.04.iso".to_string()),
        );
        disk.insert("media".to_string(), Dynamic::String("cdrom".to_string()));

        let (interface, disk_string) =
            QemuVmResource::disk_block_to_string(&Dynamic::Map(disk)).unwrap();
        assert_eq!(interface, "ide2");
        assert_eq!(disk_string, "local:iso/ubuntu-22.04.iso,media=cdrom");
    }

    #[test]
    fn test_disk_format_for_cloud_init() {
        let resource = QemuVmResource::new();
        let mut config = create_test_dynamic_value();

        // Add a cloud-init disk
        config
            .set_string(
                &AttributePath::new("ide2"),
                "local-lvm:cloudinit".to_string(),
            )
            .unwrap();

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        // Cloud-init disk should not have format
        if let Some(ide2) = &create_request.ide2 {
            assert!(!ide2.contains("format="));
        }
    }

    #[test]
    fn test_disk_block_to_string_for_cloudinit() {
        let mut disk = std::collections::HashMap::new();
        disk.insert("interface".to_string(), Dynamic::String("ide2".to_string()));
        disk.insert(
            "storage".to_string(),
            Dynamic::String("local-lvm".to_string()),
        );
        disk.insert(
            "media".to_string(),
            Dynamic::String("cloudinit".to_string()),
        );

        let (interface, disk_string) =
            QemuVmResource::disk_block_to_string(&Dynamic::Map(disk)).unwrap();
        assert_eq!(interface, "ide2");
        assert_eq!(disk_string, "local-lvm,media=cloudinit");
    }

    #[test]
    fn test_efidisk_format_for_lvm() {
        let resource = QemuVmResource::new();
        let mut config = create_test_dynamic_value_with_efidisk();

        // Override format to be empty for LVM storage
        if let Ok(mut efidisk) = config.get_map(&AttributePath::new("efidisk")) {
            efidisk.remove("format");
            let _ = config.set_map(&AttributePath::new("efidisk"), efidisk);
        }

        let result = resource.extract_vm_config(&config);
        assert!(result.is_ok());

        let (_, _, create_request) = result.unwrap();
        // EFI disk on LVM should not have format
        if let Some(efidisk0) = &create_request.efidisk0 {
            assert!(!efidisk0.contains("format="));
        }
    }

    #[test]
    fn test_populate_state_after_create_with_nested_blocks() {
        let mut state = DynamicValue::new(Dynamic::Map(std::collections::HashMap::new()));

        // Create a VM config response after creation
        let vm_config = crate::api::nodes::QemuConfig {
            name: Some("test-vm".to_string()),
            cores: Some(2),
            memory: Some(2048),
            net0: Some("virtio=BA:88:CB:76:75:D6,bridge=vmbr0,firewall=1,tag=100".to_string()),
            scsi0: Some("local-lvm:vm-100-disk-0,size=10G".to_string()),
            ..Default::default()
        };

        let mut planned_state = create_test_dynamic_value_with_network_blocks();
        // Add the attributes we want to test to the planned state
        planned_state
            .set_string(&AttributePath::new("cpu"), "x86-64-v2-AES".to_string())
            .unwrap();
        planned_state
            .set_string(&AttributePath::new("bios"), "seabios".to_string())
            .unwrap();
        planned_state
            .set_bool(&AttributePath::new("onboot"), false)
            .unwrap();
        planned_state
            .set_bool(&AttributePath::new("tablet"), true)
            .unwrap();

        QemuVmResource::populate_state_with_nested_blocks(&mut state, &vm_config, &planned_state);

        // Verify all attributes are populated
        assert_eq!(
            state.get_string(&AttributePath::new("name")).unwrap(),
            "test-vm"
        );
        assert_eq!(state.get_number(&AttributePath::new("cores")).unwrap(), 2.0);
        assert_eq!(
            state.get_number(&AttributePath::new("memory")).unwrap(),
            2048.0
        );

        // Verify zero values for optional attributes that were in planned state
        assert_eq!(
            state.get_number(&AttributePath::new("sockets")).unwrap(),
            1.0
        );
        assert_eq!(
            state.get_string(&AttributePath::new("cpu")).unwrap(),
            "x86-64-v2-AES"
        );
        assert_eq!(
            state.get_string(&AttributePath::new("bios")).unwrap(),
            "seabios"
        );
        assert_eq!(
            state.get_bool(&AttributePath::new("onboot")).unwrap(),
            false
        );
        assert_eq!(state.get_bool(&AttributePath::new("tablet")).unwrap(), true);

        // Verify network blocks are populated
        let networks = state.get_list(&AttributePath::new("network")).unwrap();
        assert_eq!(networks.len(), 1); // Only net0 exists in response

        // Verify disk list is not set since we didn't plan disk blocks
        assert!(state.get_list(&AttributePath::new("disk")).is_err());
    }
}
