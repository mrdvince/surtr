use async_trait::async_trait;
use std::collections::HashMap;
use tfplug::context::Context;
use tfplug::defaults::StaticDefault;
use tfplug::resource::{
    ConfigureResourceRequest, ConfigureResourceResponse, CreateResourceRequest,
    CreateResourceResponse, DeleteResourceRequest, DeleteResourceResponse,
    ImportResourceStateRequest, ImportResourceStateResponse, ImportedResource, ReadResourceRequest,
    ReadResourceResponse, Resource, ResourceMetadataRequest, ResourceMetadataResponse,
    ResourceSchemaRequest, ResourceSchemaResponse, ResourceWithConfigure, ResourceWithImportState,
    UpdateResourceRequest, UpdateResourceResponse, ValidateResourceConfigRequest,
    ValidateResourceConfigResponse,
};
use tfplug::schema::{
    AttributeBuilder, AttributeType, Block, NestedBlock, NestingMode, SchemaBuilder,
};
use tfplug::types::{AttributePath, Diagnostic, Dynamic, DynamicValue};

#[derive(Default)]
pub struct QemuVmResource {
    provider_data: Option<crate::ProxmoxProviderData>,
}

impl QemuVmResource {
    pub fn new() -> Self {
        Self::default()
    }

    fn normalize_tags(tags: &str) -> String {
        tags.replace(';', ",")
    }

    fn network_blocks_to_string(networks: &[Dynamic]) -> Result<String, String> {
        if networks.is_empty() {
            return Err("No network data provided".to_string());
        }

        let net_map = match &networks[0] {
            Dynamic::Map(map) => map,
            _ => return Err("Network must be a map".to_string()),
        };

        let model = net_map
            .get("model")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("virtio");

        let mut parts = vec![model.to_string()];

        if let Some(Dynamic::String(bridge)) = net_map.get("bridge") {
            parts.push(format!("bridge={}", bridge));
        }

        if let Some(Dynamic::Bool(firewall)) = net_map.get("firewall") {
            parts.push(format!("firewall={}", if *firewall { "1" } else { "0" }));
        }

        if let Some(Dynamic::Number(tag)) = net_map.get("tag") {
            parts.push(format!("tag={}", *tag as i64));
        }

        if let Some(Dynamic::String(macaddr)) = net_map.get("macaddr") {
            parts.push(format!("macaddr={}", macaddr));
        }

        if let Some(Dynamic::Number(rate)) = net_map.get("rate") {
            parts.push(format!("rate={}", rate));
        }

        if let Some(Dynamic::Number(queues)) = net_map.get("queues") {
            parts.push(format!("queues={}", *queues as i64));
        }

        if let Some(Dynamic::Bool(link_down)) = net_map.get("link_down") {
            if *link_down {
                parts.push("link_down=1".to_string());
            }
        }

        if let Some(Dynamic::Number(mtu)) = net_map.get("mtu") {
            parts.push(format!("mtu={}", *mtu as i64));
        }

        Ok(parts.join(","))
    }

    fn parse_network_string(net_string: &str, id: u32) -> Dynamic {
        let mut map = std::collections::HashMap::new();
        map.insert("id".to_string(), Dynamic::Number(id as f64));

        // Handle model type with MAC address (e.g., "virtio=BA:88:CB:76:75:D6,bridge=vmbr0")
        let parts: Vec<&str> = net_string.split(',').collect();
        let mut model = "virtio";
        let mut macaddr = None;

        // First check if the first part is model=macaddr
        if let Some(first_part) = parts.first() {
            if let Some((key, value)) = first_part.split_once('=') {
                if key == "virtio" || key == "e1000" || key == "rtl8139" || key == "vmxnet3" {
                    model = key;
                    if value.contains(':') {
                        macaddr = Some(value);
                    }
                }
            } else if first_part == &"virtio"
                || first_part == &"e1000"
                || first_part == &"rtl8139"
                || first_part == &"vmxnet3"
            {
                model = first_part;
            }
        }

        for part in parts {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "bridge" => {
                        map.insert("bridge".to_string(), Dynamic::String(value.to_string()));
                    }
                    "firewall" => {
                        let firewall = value == "1" || value == "true";
                        map.insert("firewall".to_string(), Dynamic::Bool(firewall));
                    }
                    "tag" => {
                        if let Ok(tag) = value.parse::<f64>() {
                            map.insert("tag".to_string(), Dynamic::Number(tag));
                        }
                    }
                    "macaddr" => {
                        map.insert("macaddr".to_string(), Dynamic::String(value.to_string()));
                    }
                    "rate" => {
                        if let Ok(rate) = value.parse::<f64>() {
                            map.insert("rate".to_string(), Dynamic::Number(rate));
                        }
                    }
                    "queues" => {
                        if let Ok(queues) = value.parse::<f64>() {
                            map.insert("queues".to_string(), Dynamic::Number(queues));
                        }
                    }
                    "link_down" => {
                        let link_down = value == "1" || value == "true";
                        map.insert("link_down".to_string(), Dynamic::Bool(link_down));
                    }
                    "mtu" => {
                        if let Ok(mtu) = value.parse::<f64>() {
                            map.insert("mtu".to_string(), Dynamic::Number(mtu));
                        }
                    }
                    _ => {}
                }
            }
        }

        map.insert("model".to_string(), Dynamic::String(model.to_string()));
        if let Some(mac) = macaddr {
            map.insert("macaddr".to_string(), Dynamic::String(mac.to_string()));
        }
        Dynamic::Map(map)
    }

    fn parse_disk_string(disk_string: &str, slot: &str) -> Dynamic {
        let mut map = std::collections::HashMap::new();
        map.insert("slot".to_string(), Dynamic::String(slot.to_string()));

        // Determine type from slot (e.g., scsi0 -> scsi, virtio0 -> virtio)
        let disk_type = if slot.starts_with("scsi") {
            "scsi"
        } else if slot.starts_with("virtio") {
            "virtio"
        } else if slot.starts_with("ide") {
            "ide"
        } else if slot.starts_with("sata") {
            "sata"
        } else {
            "unknown"
        };
        map.insert("type".to_string(), Dynamic::String(disk_type.to_string()));

        let parts: Vec<&str> = disk_string.split(',').collect();

        if let Some(storage_part) = parts.first() {
            if let Some((storage, path_or_size)) = storage_part.split_once(':') {
                map.insert("storage".to_string(), Dynamic::String(storage.to_string()));

                if path_or_size.contains("iso/") {
                    map.insert("iso".to_string(), Dynamic::String(path_or_size.to_string()));
                } else if path_or_size == "cloudinit" {
                } else if path_or_size.chars().all(|c| c.is_numeric()) {
                    let size_str = format!("{}G", path_or_size);
                    map.insert("size".to_string(), Dynamic::String(size_str));
                }
            } else {
                map.insert(
                    "storage".to_string(),
                    Dynamic::String(storage_part.to_string()),
                );
            }
        }

        let size_found = map.contains_key("size");
        if !size_found {
            for part in &parts {
                if let Some((key, value)) = part.split_once('=') {
                    if key == "size" {
                        map.insert("size".to_string(), Dynamic::String(value.to_string()));
                        break;
                    }
                }
            }
        }

        for part in parts.iter().skip(1) {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "media" => {
                        map.insert("media".to_string(), Dynamic::String(value.to_string()));
                    }
                    "format" => {
                        map.insert("format".to_string(), Dynamic::String(value.to_string()));
                    }
                    "iothread" => {
                        let iothread = value == "1" || value == "true";
                        map.insert("iothread".to_string(), Dynamic::Bool(iothread));
                    }
                    "ssd" => {
                        let ssd = value == "1" || value == "true";
                        map.insert("emulatessd".to_string(), Dynamic::Bool(ssd));
                    }
                    "discard" => {
                        let discard = value == "on" || value == "1";
                        map.insert("discard".to_string(), Dynamic::Bool(discard));
                    }
                    "cache" => {
                        map.insert("cache".to_string(), Dynamic::String(value.to_string()));
                    }
                    "backup" => {
                        let backup = value == "1" || value == "true";
                        map.insert("backup".to_string(), Dynamic::Bool(backup));
                    }
                    "replicate" => {
                        let replicate = value == "1" || value == "true";
                        map.insert("replicate".to_string(), Dynamic::Bool(replicate));
                    }
                    _ => {}
                }
            }
        }

        Dynamic::Map(map)
    }

    fn normalize_network_config(net_config: &str, current_config: Option<&str>) -> String {
        let should_remove_mac = current_config.map(|c| !c.contains(':')).unwrap_or(true);

        let parts: Vec<&str> = net_config.split(',').collect();
        let mut network_type = None;
        let mut params = Vec::new();

        for part in parts {
            if let Some((key, value)) = part.split_once('=') {
                if key == "virtio" || key == "e1000" || key == "rtl8139" || key == "vmxnet3" {
                    if value.contains(':') && should_remove_mac {
                        network_type = Some(key.to_string());
                    } else {
                        network_type = Some(part.to_string());
                    }
                } else {
                    params.push((key, value));
                }
            } else {
                // Handle cases where there's no '=' (like standalone virtio)
                if part == "virtio" || part == "e1000" || part == "rtl8139" || part == "vmxnet3" {
                    network_type = Some(part.to_string());
                } else {
                    params.push((part, ""));
                }
            }
        }

        // Sort parameters alphabetically by key
        params.sort_by(|a, b| a.0.cmp(b.0));

        // Reconstruct the config string
        let mut result = Vec::new();
        if let Some(nt) = network_type {
            result.push(nt);
        }

        for (key, value) in params {
            if value.is_empty() {
                result.push(key.to_string());
            } else {
                result.push(format!("{}={}", key, value));
            }
        }

        result.join(",")
    }

    fn normalize_disk_config(disk_config: &str, current_config: Option<&str>) -> String {
        // Proxmox returns disk configs like "local-lvm:vm-9003-disk-1,size=10G"
        // But Terraform expects "local-lvm:10,format=raw"

        if let Some(current) = current_config {
            // If current config has a simple format like "storage:size,format=raw"
            // we should return that instead of the detailed Proxmox response
            if current.contains("format=") && !current.contains("vm-") {
                return current.to_string();
            }
        }

        // Otherwise return the disk config as-is
        disk_config.to_string()
    }

    fn validate_iothread(&self, config: &DynamicValue, diagnostics: &mut Vec<Diagnostic>) {
        // Check SCSI disks with iothread
        for i in 0..=30 {
            let disk_key = format!("scsi{}", i);
            if let Ok(disk_config) = config.get_string(&AttributePath::new(&disk_key)) {
                if disk_config.contains("iothread=1") || disk_config.contains("iothread=true") {
                    // Check if scsihw is virtio-scsi-single
                    let scsihw = config
                        .get_string(&AttributePath::new("scsihw"))
                        .unwrap_or_else(|_| "lsi".to_string());

                    if scsihw != "virtio-scsi-single" {
                        diagnostics.push(Diagnostic::warning(
                            "iothread requires virtio-scsi-single",
                            format!("Disk {} has iothread enabled but scsihw is '{}'. iothread is only valid with scsihw='virtio-scsi-single'", disk_key, scsihw),
                        ));
                    }
                }
            }
        }

        // Check SATA disks (iothread not supported)
        for i in 0..=5 {
            let disk_key = format!("sata{}", i);
            if let Ok(disk_config) = config.get_string(&AttributePath::new(&disk_key)) {
                if disk_config.contains("iothread=1") || disk_config.contains("iothread=true") {
                    diagnostics.push(Diagnostic::warning(
                        "iothread not supported for SATA",
                        format!("Disk {} has iothread enabled but iothread is not supported for SATA disks. Use SCSI with virtio-scsi-single or VirtIO disks instead.", disk_key),
                    ));
                }
            }
        }

        // Check IDE disks (iothread not supported)
        for i in 0..=3 {
            let disk_key = format!("ide{}", i);
            if let Ok(disk_config) = config.get_string(&AttributePath::new(&disk_key)) {
                if disk_config.contains("iothread=1") || disk_config.contains("iothread=true") {
                    diagnostics.push(Diagnostic::warning(
                        "iothread not supported for IDE",
                        format!("Disk {} has iothread enabled but iothread is not supported for IDE disks. Use SCSI with virtio-scsi-single or VirtIO disks instead.", disk_key),
                    ));
                }
            }
        }
    }

    // Block conversion methods for nested block attributes
    fn disk_block_to_api_string(disk: &Dynamic) -> Result<(String, String), String> {
        let disk_map = match disk {
            Dynamic::Map(map) => map,
            _ => return Err("Disk must be a map".to_string()),
        };

        let slot = disk_map
            .get("slot")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or("Slot is required")?;

        let storage = disk_map
            .get("storage")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.as_str()),
                _ => None,
            })
            .ok_or("Storage is required")?;

        let size = disk_map
            .get("size")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.as_str()),
                _ => None,
            })
            .ok_or("Size is required")?;

        // Convert size format (e.g., "20G" to "20")
        let size_num = size.trim_end_matches('G').trim_end_matches('g');
        let mut parts = vec![format!("{}:{}", storage, size_num)];

        // Add optional attributes
        if let Some(Dynamic::String(format)) = disk_map.get("format") {
            if !format.is_empty() {
                parts.push(format!("format={}", format));
            }
        }

        if let Some(Dynamic::Bool(true)) = disk_map.get("iothread") {
            parts.push("iothread=1".to_string());
        }

        if let Some(Dynamic::Bool(true)) = disk_map.get("emulatessd") {
            parts.push("ssd=1".to_string());
        }

        if let Some(Dynamic::Bool(true)) = disk_map.get("discard") {
            parts.push("discard=on".to_string());
        }

        if let Some(Dynamic::Bool(false)) = disk_map.get("backup") {
            parts.push("backup=0".to_string());
        }

        if let Some(Dynamic::Bool(false)) = disk_map.get("replicate") {
            parts.push("replicate=0".to_string());
        }

        if let Some(Dynamic::Bool(true)) = disk_map.get("readonly") {
            parts.push("ro=1".to_string());
        }

        // IO limits
        if let Some(Dynamic::Number(n)) = disk_map.get("iops_r_burst") {
            parts.push(format!("iops_rd_max={}", *n as i64));
        }
        if let Some(Dynamic::Number(n)) = disk_map.get("iops_r_concurrent") {
            parts.push(format!("iops_rd={}", *n as i64));
        }
        if let Some(Dynamic::Number(n)) = disk_map.get("iops_wr_burst") {
            parts.push(format!("iops_wr_max={}", *n as i64));
        }
        if let Some(Dynamic::Number(n)) = disk_map.get("iops_wr_concurrent") {
            parts.push(format!("iops_wr={}", *n as i64));
        }

        // Bandwidth limits
        if let Some(Dynamic::Number(n)) = disk_map.get("mbps_r_burst") {
            parts.push(format!("mbps_rd_max={}", *n as i64));
        }
        if let Some(Dynamic::Number(n)) = disk_map.get("mbps_r_concurrent") {
            parts.push(format!("mbps_rd={}", *n as i64));
        }
        if let Some(Dynamic::Number(n)) = disk_map.get("mbps_wr_burst") {
            parts.push(format!("mbps_wr_max={}", *n as i64));
        }
        if let Some(Dynamic::Number(n)) = disk_map.get("mbps_wr_concurrent") {
            parts.push(format!("mbps_wr={}", *n as i64));
        }

        Ok((slot, parts.join(",")))
    }

    fn cdrom_block_to_api_string(cdrom: &Dynamic) -> Result<(String, String), String> {
        let cdrom_map = match cdrom {
            Dynamic::Map(map) => map,
            _ => return Err("CD-ROM must be a map".to_string()),
        };

        let slot = cdrom_map
            .get("slot")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or("Slot is required")?;

        let iso = cdrom_map
            .get("iso")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or("ISO is required")?;

        Ok((slot, format!("{},media=cdrom", iso)))
    }

    fn cloudinit_drive_block_to_api_string(ci_drive: &Dynamic) -> Result<(String, String), String> {
        let ci_map = match ci_drive {
            Dynamic::Map(map) => map,
            _ => return Err("Cloud-init drive must be a map".to_string()),
        };

        let slot = ci_map
            .get("slot")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or("Slot is required")?;

        let storage = ci_map
            .get("storage")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or("Storage is required")?;

        Ok((slot, format!("{}:cloudinit", storage)))
    }

    fn serial_block_to_api_string(serial: &Dynamic) -> Result<(u32, String), String> {
        let serial_map = match serial {
            Dynamic::Map(map) => map,
            _ => return Err("Serial must be a map".to_string()),
        };

        let id = serial_map
            .get("id")
            .and_then(|v| match v {
                Dynamic::Number(n) => Some(*n as u32),
                _ => None,
            })
            .ok_or("ID is required")?;

        let type_str = serial_map
            .get("type")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or("Type is required")?;

        Ok((id, type_str))
    }

    fn efidisk_block_to_api_string(efidisk: &Dynamic) -> Result<String, String> {
        let efidisk_map = match efidisk {
            Dynamic::Map(map) => map,
            _ => return Err("EFI disk must be a map".to_string()),
        };

        let storage = efidisk_map
            .get("storage")
            .and_then(|v| match v {
                Dynamic::String(s) => Some(s.as_str()),
                _ => None,
            })
            .ok_or("Storage is required")?;

        // Default size for EFI disk
        let mut parts = vec![format!("{}:1", storage)];

        if let Some(Dynamic::String(efitype)) = efidisk_map.get("efitype") {
            parts.push(format!("efitype={}", efitype));
        }

        Ok(parts.join(","))
    }
}

#[async_trait]
impl Resource for QemuVmResource {
    fn type_name(&self) -> &str {
        "proxmox_qemu_vm"
    }

    async fn metadata(
        &self,
        _ctx: Context,
        _request: ResourceMetadataRequest,
    ) -> ResourceMetadataResponse {
        ResourceMetadataResponse {
            type_name: self.type_name().to_string(),
        }
    }

    async fn schema(
        &self,
        _ctx: Context,
        _request: ResourceSchemaRequest,
    ) -> ResourceSchemaResponse {
        let schema = SchemaBuilder::new()
            .version(0)
            .description("Manages QEMU/KVM virtual machines in Proxmox VE")
            // Core VM Identity
            .attribute(
                AttributeBuilder::new("vmid", AttributeType::Number)
                    .description("The VM identifier")
                    .required()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("name", AttributeType::String)
                    .description("The VM name")
                    .required()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("target_node", AttributeType::String)
                    .description("The name of the Proxmox node where the VM will be created")
                    .required()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("tags", AttributeType::String)
                    .description("Tags for the VM (separated by semicolon or comma)")
                    .optional()
                    .build(),
            )
            // Clone/Template Settings
            .attribute(
                AttributeBuilder::new("clone", AttributeType::String)
                    .description("Name of the template to clone from")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("full_clone", AttributeType::Bool)
                    .description("Create a full copy of all disk/container data")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("os_type", AttributeType::String)
                    .description("OS type for optimized settings")
                    .optional()
                    .build(),
            )
            // Hardware Configuration
            .attribute(
                AttributeBuilder::new("bios", AttributeType::String)
                    .description("BIOS implementation (seabios or ovmf)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("machine", AttributeType::String)
                    .description("Machine type (e.g., pc, q35)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("cpu_type", AttributeType::String)
                    .description("CPU type")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("cores", AttributeType::Number)
                    .description("Number of CPU cores per socket")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("sockets", AttributeType::Number)
                    .description("Number of CPU sockets")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("vcpus", AttributeType::Number)
                    .description("Number of vCPUs")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("memory", AttributeType::Number)
                    .description("Amount of RAM for the VM in MB")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("balloon", AttributeType::Number)
                    .description("Amount of target RAM for the VM in MB")
                    .optional()
                    .build(),
            )
            // Boot Configuration
            .attribute(
                AttributeBuilder::new("boot", AttributeType::String)
                    .description("Boot order")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("bootdisk", AttributeType::String)
                    .description("Enable booting from specified disk")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("onboot", AttributeType::Bool)
                    .description("Start VM on boot")
                    .optional()
                    .build(),
            )
            // Storage Configuration
            .attribute(
                AttributeBuilder::new("scsihw", AttributeType::String)
                    .description("SCSI controller type")
                    .optional()
                    .build(),
            )
            // Guest Agent & OS Settings
            .attribute(
                AttributeBuilder::new("agent", AttributeType::Number)
                    .description("Enable/disable the QEMU guest agent")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("qemu_os", AttributeType::String)
                    .description("QEMU OS type")
                    .optional()
                    .build(),
            )
            // Cloud-Init Configuration
            .attribute(
                AttributeBuilder::new("ipconfig0", AttributeType::String)
                    .description("Cloud-init network configuration for interface 0")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig1", AttributeType::String)
                    .description("Cloud-init network configuration for interface 1")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig2", AttributeType::String)
                    .description("Cloud-init network configuration for interface 2")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig3", AttributeType::String)
                    .description("Cloud-init network configuration for interface 3")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ciuser", AttributeType::String)
                    .description("Cloud-init user")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("cipassword", AttributeType::String)
                    .description("Cloud-init password")
                    .sensitive()
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ciupgrade", AttributeType::Bool)
                    .description("Do an automatic package upgrade after the first boot")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("sshkeys", AttributeType::String)
                    .description("Cloud-init SSH public keys")
                    .optional()
                    .build(),
            )
            // Network Settings
            .attribute(
                AttributeBuilder::new("skip_ipv4", AttributeType::Bool)
                    .description("Skip IPv4 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("skip_ipv6", AttributeType::Bool)
                    .description("Skip IPv6 configuration")
                    .optional()
                    .build(),
            )
            // Timing & Behavior Settings
            .attribute(
                AttributeBuilder::new("additional_wait", AttributeType::Number)
                    .description("Additional wait time after VM creation")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("automatic_reboot", AttributeType::Bool)
                    .description("Automatically reboot VM after creation")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("clone_wait", AttributeType::Number)
                    .description("Wait time for clone operation")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("define_connection_info", AttributeType::Bool)
                    .description("Define connection info for provisioners")
                    .optional()
                    .build(),
            )
            // Other attributes
            .attribute(
                AttributeBuilder::new("description", AttributeType::String)
                    .description("VM description")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("start", AttributeType::Bool)
                    .description("Start VM after creation")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("tablet", AttributeType::Bool)
                    .description("Enable tablet device")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("protection", AttributeType::Bool)
                    .description("Protection flag to prevent accidental deletion")
                    .optional()
                    .build(),
            )
            .block(NestedBlock {
                type_name: "network".to_string(),
                block: Block {
                    version: 0,
                    attributes: vec![
                        AttributeBuilder::new("id", AttributeType::Number)
                            .required()
                            .description("Network interface ID (0-31)")
                            .build(),
                        AttributeBuilder::new("model", AttributeType::String)
                            .optional()
                            .description("Network card model: virtio, e1000, rtl8139, vmxnet3")
                            .default(StaticDefault::create(Dynamic::String("virtio".to_string())))
                            .build(),
                        AttributeBuilder::new("bridge", AttributeType::String)
                            .required()
                            .description("Bridge to attach the network interface to")
                            .build(),
                        AttributeBuilder::new("firewall", AttributeType::Bool)
                            .optional()
                            .description("Enable firewall on this interface")
                            .default(StaticDefault::create(Dynamic::Bool(false)))
                            .build(),
                        AttributeBuilder::new("tag", AttributeType::Number)
                            .optional()
                            .description("VLAN tag (1-4094)")
                            .default(StaticDefault::create(Dynamic::Number(-1.0)))
                            .build(),
                        AttributeBuilder::new("macaddr", AttributeType::String)
                            .optional()
                            .computed()
                            .description("MAC address (computed if not provided)")
                            .default(StaticDefault::create(Dynamic::String("".to_string())))
                            .build(),
                        AttributeBuilder::new("rate", AttributeType::Number)
                            .optional()
                            .description("Rate limit in MB/s")
                            .default(StaticDefault::create(Dynamic::Number(-1.0)))
                            .build(),
                        AttributeBuilder::new("queues", AttributeType::Number)
                            .optional()
                            .description("Number of packet queues (1-64)")
                            .default(StaticDefault::create(Dynamic::Number(-1.0)))
                            .build(),
                        AttributeBuilder::new("link_down", AttributeType::Bool)
                            .optional()
                            .description("Link down (disconnect)")
                            .default(StaticDefault::create(Dynamic::Bool(false)))
                            .build(),
                        AttributeBuilder::new("mtu", AttributeType::Number)
                            .optional()
                            .description("MTU (576-65536)")
                            .build(),
                    ],
                    block_types: vec![],
                    description: "Network interface configuration".to_string(),
                    description_kind: tfplug::schema::StringKind::Plain,
                    deprecated: false,
                },
                nesting: NestingMode::List,
                min_items: 0,
                max_items: 32,
            })
            // Disk Configuration Block
            .block(NestedBlock {
                type_name: "disk".to_string(),
                block: Block {
                    version: 0,
                    attributes: vec![
                        AttributeBuilder::new("slot", AttributeType::String)
                            .required()
                            .description("Disk slot (e.g., scsi0, virtio0, ide0)")
                            .build(),
                        AttributeBuilder::new("type", AttributeType::String)
                            .required()
                            .description("Disk type: scsi, virtio, ide, sata")
                            .build(),
                        AttributeBuilder::new("storage", AttributeType::String)
                            .required()
                            .description("Storage pool name")
                            .build(),
                        AttributeBuilder::new("size", AttributeType::String)
                            .required()
                            .description("Disk size (e.g., 10G, 1T)")
                            .build(),
                        AttributeBuilder::new("format", AttributeType::String)
                            .optional()
                            .description("Disk format (raw, qcow2, vmdk)")
                            .build(),
                        // Performance Settings
                        AttributeBuilder::new("discard", AttributeType::Bool)
                            .optional()
                            .description("Enable discard/trim")
                            .build(),
                        AttributeBuilder::new("emulatessd", AttributeType::Bool)
                            .optional()
                            .description("Emulate SSD drive")
                            .build(),
                        AttributeBuilder::new("iothread", AttributeType::Bool)
                            .optional()
                            .description("Enable iothread")
                            .build(),
                        // Data Protection
                        AttributeBuilder::new("backup", AttributeType::Bool)
                            .optional()
                            .description("Include in backups")
                            .build(),
                        AttributeBuilder::new("replicate", AttributeType::Bool)
                            .optional()
                            .description("Include in replication")
                            .build(),
                        AttributeBuilder::new("readonly", AttributeType::Bool)
                            .optional()
                            .description("Set disk as read-only")
                            .build(),
                        // IO Limits
                        AttributeBuilder::new("iops_r_burst", AttributeType::Number)
                            .optional()
                            .description("Maximum read IO burst")
                            .build(),
                        AttributeBuilder::new("iops_r_burst_length", AttributeType::Number)
                            .optional()
                            .description("Length of read IO burst in seconds")
                            .build(),
                        AttributeBuilder::new("iops_r_concurrent", AttributeType::Number)
                            .optional()
                            .description("Maximum concurrent read IO")
                            .build(),
                        AttributeBuilder::new("iops_wr_burst", AttributeType::Number)
                            .optional()
                            .description("Maximum write IO burst")
                            .build(),
                        AttributeBuilder::new("iops_wr_burst_length", AttributeType::Number)
                            .optional()
                            .description("Length of write IO burst in seconds")
                            .build(),
                        AttributeBuilder::new("iops_wr_concurrent", AttributeType::Number)
                            .optional()
                            .description("Maximum concurrent write IO")
                            .build(),
                        // Bandwidth Limits
                        AttributeBuilder::new("mbps_r_burst", AttributeType::Number)
                            .optional()
                            .description("Maximum read bandwidth burst in MB/s")
                            .build(),
                        AttributeBuilder::new("mbps_r_concurrent", AttributeType::Number)
                            .optional()
                            .description("Maximum concurrent read bandwidth in MB/s")
                            .build(),
                        AttributeBuilder::new("mbps_wr_burst", AttributeType::Number)
                            .optional()
                            .description("Maximum write bandwidth burst in MB/s")
                            .build(),
                        AttributeBuilder::new("mbps_wr_concurrent", AttributeType::Number)
                            .optional()
                            .description("Maximum concurrent write bandwidth in MB/s")
                            .build(),
                    ],
                    block_types: vec![],
                    description: "Disk configuration".to_string(),
                    description_kind: tfplug::schema::StringKind::Plain,
                    deprecated: false,
                },
                nesting: NestingMode::List,
                min_items: 0,
                max_items: 256,
            })
            // CD-ROM Configuration Block
            .block(NestedBlock {
                type_name: "cdrom".to_string(),
                block: Block {
                    version: 0,
                    attributes: vec![
                        AttributeBuilder::new("slot", AttributeType::String)
                            .required()
                            .description("CD-ROM slot (e.g., ide2)")
                            .build(),
                        AttributeBuilder::new("iso", AttributeType::String)
                            .required()
                            .description("ISO image path (e.g., local:iso/ubuntu.iso)")
                            .build(),
                    ],
                    block_types: vec![],
                    description: "CD-ROM configuration".to_string(),
                    description_kind: tfplug::schema::StringKind::Plain,
                    deprecated: false,
                },
                nesting: NestingMode::List,
                min_items: 0,
                max_items: 4,
            })
            // Cloud-Init Drive Block
            .block(NestedBlock {
                type_name: "cloudinit_drive".to_string(),
                block: Block {
                    version: 0,
                    attributes: vec![
                        AttributeBuilder::new("slot", AttributeType::String)
                            .required()
                            .description("Cloud-init drive slot (e.g., ide3)")
                            .build(),
                        AttributeBuilder::new("storage", AttributeType::String)
                            .required()
                            .description("Storage pool for cloud-init drive")
                            .build(),
                    ],
                    block_types: vec![],
                    description: "Cloud-init drive configuration".to_string(),
                    description_kind: tfplug::schema::StringKind::Plain,
                    deprecated: false,
                },
                nesting: NestingMode::List,
                min_items: 0,
                max_items: 1,
            })
            // Serial Port Block
            .block(NestedBlock {
                type_name: "serial".to_string(),
                block: Block {
                    version: 0,
                    attributes: vec![
                        AttributeBuilder::new("id", AttributeType::Number)
                            .required()
                            .description("Serial port ID")
                            .build(),
                        AttributeBuilder::new("type", AttributeType::String)
                            .required()
                            .description("Serial port type (e.g., socket)")
                            .build(),
                    ],
                    block_types: vec![],
                    description: "Serial port configuration".to_string(),
                    description_kind: tfplug::schema::StringKind::Plain,
                    deprecated: false,
                },
                nesting: NestingMode::List,
                min_items: 0,
                max_items: 4,
            })
            // EFI Disk Block
            .block(NestedBlock {
                type_name: "efidisk".to_string(),
                block: Block {
                    version: 0,
                    attributes: vec![
                        AttributeBuilder::new("efitype", AttributeType::String)
                            .optional()
                            .description("EFI type (2m, 4m)")
                            .default(StaticDefault::string("4m"))
                            .build(),
                        AttributeBuilder::new("storage", AttributeType::String)
                            .required()
                            .description("Storage pool name")
                            .build(),
                        AttributeBuilder::new("format", AttributeType::String)
                            .optional()
                            .description("Disk format (raw, qcow2)")
                            .default(StaticDefault::string("raw"))
                            .build(),
                        AttributeBuilder::new("pre_enrolled_keys", AttributeType::Bool)
                            .optional()
                            .description("Use pre-enrolled keys")
                            .default(StaticDefault::bool(false))
                            .build(),
                    ],
                    block_types: vec![],
                    description: "EFI disk configuration".to_string(),
                    description_kind: tfplug::schema::StringKind::Plain,
                    deprecated: false,
                },
                nesting: NestingMode::List,
                min_items: 0,
                max_items: 1,
            })
            .build();

        ResourceSchemaResponse {
            schema,
            diagnostics: vec![],
        }
    }

    async fn validate(
        &self,
        _ctx: Context,
        request: ValidateResourceConfigRequest,
    ) -> ValidateResourceConfigResponse {
        let mut diagnostics = vec![];

        if let Ok(vmid) = request.config.get_number(&AttributePath::new("vmid")) {
            let vmid_int = vmid as u32;
            if !(100..=999999999).contains(&vmid_int) {
                diagnostics.push(Diagnostic::error(
                    "Invalid VMID",
                    "VMID must be between 100 and 999999999",
                ));
            }
        }

        if let Ok(cores) = request.config.get_number(&AttributePath::new("cores")) {
            if !(1.0..=128.0).contains(&cores) {
                diagnostics.push(Diagnostic::error(
                    "Invalid cores",
                    "Cores must be between 1 and 128",
                ));
            }
        }

        if let Ok(sockets) = request.config.get_number(&AttributePath::new("sockets")) {
            if !(1.0..=4.0).contains(&sockets) {
                diagnostics.push(Diagnostic::error(
                    "Invalid sockets",
                    "Sockets must be between 1 and 4",
                ));
            }
        }

        if let Ok(memory) = request.config.get_number(&AttributePath::new("memory")) {
            if !(16.0..=8388608.0).contains(&memory) {
                diagnostics.push(Diagnostic::error(
                    "Invalid memory",
                    "Memory must be between 16 MB and 8 TB",
                ));
            }
        }

        if let Ok(bios) = request.config.get_string(&AttributePath::new("bios")) {
            if !["seabios", "ovmf"].contains(&bios.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "Invalid BIOS",
                    "BIOS must be either 'seabios' or 'ovmf'",
                ));
            }

            // Validate OVMF requires efidisk
            if bios == "ovmf" {
                // Check for efidisk0 string attribute
                let has_efidisk0 = request
                    .config
                    .get_string(&AttributePath::new("efidisk0"))
                    .is_ok();

                // Check for efidisk nested block (it's a list with max_items: 1)
                let has_efidisk_block = request
                    .config
                    .get_list(&AttributePath::new("efidisk"))
                    .map(|list| !list.is_empty())
                    .unwrap_or(false);

                if !has_efidisk0 && !has_efidisk_block {
                    diagnostics.push(Diagnostic::warning(
                        "OVMF requires EFI disk",
                        "When using OVMF BIOS, you should configure efidisk0 (e.g., efidisk0 = \"local-lvm:1,format=qcow2\") or use the efidisk block. Without it, a temporary EFI vars disk will be used.",
                    ));
                }
            }
        }

        // Validate iothread usage
        self.validate_iothread(&request.config, &mut diagnostics);

        ValidateResourceConfigResponse { diagnostics }
    }

    async fn create(
        &self,
        _ctx: Context,
        request: CreateResourceRequest,
    ) -> CreateResourceResponse {
        let mut diagnostics = vec![];

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return CreateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                };
            }
        };

        match self.extract_vm_config(&request.config) {
            Ok((node, _vmid, create_request)) => {
                match provider_data
                    .client
                    .nodes()
                    .node(&node)
                    .qemu()
                    .create(create_request.vmid, &create_request)
                    .await
                {
                    Ok(_task_id) => {
                        // Wait for VM creation to complete if additional_wait is specified
                        if let Ok(wait_time) = request
                            .config
                            .get_number(&AttributePath::new("additional_wait"))
                        {
                            if wait_time > 0.0 {
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    wait_time as u64,
                                ))
                                .await;
                            }
                        }

                        // For now, just return the planned state
                        // TODO: Fix the issue where reading the VM config returns different values than what we sent
                        // This is a temporary workaround - we should properly wait for the task to complete
                        // and then read the actual VM configuration from the API
                        CreateResourceResponse {
                            new_state: request.planned_state.clone(),
                            private: vec![],
                            diagnostics,
                        }
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            "Failed to create VM",
                            format!("API error: {}", e),
                        ));
                        // Return planned state with all attributes populated to avoid "missing attribute" errors
                        let mut failed_state = request.planned_state.clone();

                        // Ensure all required attributes are present even on failure
                        Self::populate_all_attributes(&mut failed_state, &request.planned_state);

                        CreateResourceResponse {
                            new_state: failed_state,
                            private: vec![],
                            diagnostics,
                        }
                    }
                }
            }
            Err(diag) => {
                diagnostics.push(diag);
                // Return planned state with all attributes populated to avoid "missing attribute" errors
                let mut failed_state = request.planned_state.clone();
                Self::populate_all_attributes(&mut failed_state, &request.planned_state);

                CreateResourceResponse {
                    new_state: failed_state,
                    private: vec![],
                    diagnostics,
                }
            }
        }
    }

    async fn read(&self, _ctx: Context, request: ReadResourceRequest) -> ReadResourceResponse {
        let mut diagnostics = vec![];

        let node = match request
            .current_state
            .get_string(&AttributePath::new("target_node"))
        {
            Ok(node) => node,
            Err(_) => {
                return ReadResourceResponse {
                    new_state: None,
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        let vmid = match request
            .current_state
            .get_number(&AttributePath::new("vmid"))
        {
            Ok(vmid) => vmid as u32,
            Err(_) => {
                return ReadResourceResponse {
                    new_state: None,
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return ReadResourceResponse {
                    new_state: Some(request.current_state),
                    private: request.private,
                    diagnostics,
                    deferred: None,
                    new_identity: None,
                };
            }
        };

        match provider_data
            .client
            .nodes()
            .node(&node)
            .qemu()
            .get_config(vmid)
            .await
        {
            Ok(vm_config) => {
                let mut new_state = request.current_state.clone();

                // Check if we have nested blocks in the current state
                let has_network_blocks = request
                    .current_state
                    .get_list(&AttributePath::new("network"))
                    .is_ok();
                let has_disk_blocks = request
                    .current_state
                    .get_list(&AttributePath::new("disk"))
                    .is_ok();
                let has_efidisk_block = request
                    .current_state
                    .get_list(&AttributePath::new("efidisk"))
                    .map(|list| !list.is_empty())
                    .unwrap_or(false);

                if has_network_blocks || has_disk_blocks || has_efidisk_block {
                    Self::populate_state_with_nested_blocks(
                        &mut new_state,
                        &vm_config,
                        &request.current_state,
                    );
                } else {
                    Self::populate_state_from_config(
                        &mut new_state,
                        &vm_config,
                        &request.current_state,
                    );
                }

                ReadResourceResponse {
                    new_state: Some(new_state),
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
            Err(crate::api::ApiError::ApiError {
                status, message, ..
            }) if status == 404
                || message.contains("does not exist")
                || message.contains("not found") =>
            {
                ReadResourceResponse {
                    new_state: None,
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
            Err(crate::api::ApiError::ServiceUnavailable) => {
                // When a VM doesn't exist, Proxmox might return ServiceUnavailable
                // We should check if the VM actually exists by listing VMs
                match provider_data.client.nodes().node(&node).qemu().list().await {
                    Ok(vms) => {
                        if vms.iter().any(|vm| vm.vmid == vmid) {
                            // VM exists but service is temporarily unavailable
                            diagnostics.push(Diagnostic::error(
                                "Failed to read VM",
                                "Service temporarily unavailable, please try again",
                            ));
                            ReadResourceResponse {
                                new_state: Some(request.current_state),
                                diagnostics,
                                private: request.private,
                                deferred: None,
                                new_identity: None,
                            }
                        } else {
                            // VM doesn't exist, remove from state
                            ReadResourceResponse {
                                new_state: None,
                                diagnostics,
                                private: request.private,
                                deferred: None,
                                new_identity: None,
                            }
                        }
                    }
                    Err(_) => {
                        // Can't determine if VM exists, keep current state and report error
                        diagnostics.push(Diagnostic::error(
                            "Failed to read VM",
                            "Service unavailable and unable to verify VM existence",
                        ));
                        ReadResourceResponse {
                            new_state: Some(request.current_state),
                            diagnostics,
                            private: request.private,
                            deferred: None,
                            new_identity: None,
                        }
                    }
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to read VM",
                    format!("API error: {}", e),
                ));
                ReadResourceResponse {
                    new_state: Some(request.current_state),
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
        }
    }

    async fn update(
        &self,
        _ctx: Context,
        request: UpdateResourceRequest,
    ) -> UpdateResourceResponse {
        let mut diagnostics = vec![];

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Provider data was not properly configured",
                ));
                return UpdateResourceResponse {
                    new_state: request.planned_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        let node = match request
            .config
            .get_string(&AttributePath::new("target_node"))
        {
            Ok(node) => node,
            Err(diag) => {
                diagnostics.push(Diagnostic::error("Missing node", diag.to_string()));
                return UpdateResourceResponse {
                    new_state: request.prior_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        let vmid = match request.config.get_number(&AttributePath::new("vmid")) {
            Ok(vmid) => vmid as u32,
            Err(diag) => {
                diagnostics.push(Diagnostic::error("Missing vmid", diag.to_string()));
                return UpdateResourceResponse {
                    new_state: request.prior_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                };
            }
        };

        match self.build_update_request(&request.config) {
            Ok(update_request) => {
                match provider_data
                    .client
                    .nodes()
                    .node(&node)
                    .qemu()
                    .update_config(vmid, &update_request)
                    .await
                {
                    Ok(_) => UpdateResourceResponse {
                        new_state: request.planned_state,
                        private: vec![],
                        diagnostics,
                        new_identity: None,
                    },
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            "Failed to update VM",
                            format!("API error: {}", e),
                        ));
                        UpdateResourceResponse {
                            new_state: request.prior_state,
                            private: vec![],
                            diagnostics,
                            new_identity: None,
                        }
                    }
                }
            }
            Err(diag) => {
                diagnostics.push(diag);
                UpdateResourceResponse {
                    new_state: request.prior_state,
                    private: vec![],
                    diagnostics,
                    new_identity: None,
                }
            }
        }
    }

    async fn delete(
        &self,
        _ctx: Context,
        request: DeleteResourceRequest,
    ) -> DeleteResourceResponse {
        let mut diagnostics = vec![];

        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                return DeleteResourceResponse { diagnostics };
            }
        };

        let node = match request
            .prior_state
            .get_string(&AttributePath::new("target_node"))
        {
            Ok(node) => node,
            Err(_) => {
                return DeleteResourceResponse { diagnostics };
            }
        };

        let vmid = match request.prior_state.get_number(&AttributePath::new("vmid")) {
            Ok(vmid) => vmid as u32,
            Err(_) => {
                return DeleteResourceResponse { diagnostics };
            }
        };

        // Check if VM is running before attempting deletion
        let qemu_api = provider_data.client.nodes().node(&node).qemu();

        match qemu_api.get_status(vmid).await {
            Ok(status) => {
                // If VM is running, stop it first
                if status.status == "running" {
                    match qemu_api.stop(vmid).await {
                        Ok(_) => {
                            // Wait for VM to stop (5 seconds should be enough for most cases)
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                        Err(e) => {
                            diagnostics.push(Diagnostic::warning(
                                "Failed to stop VM",
                                format!("Could not stop VM before deletion: {}. Attempting deletion anyway.", e),
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                // If we can't get status, log a warning but proceed with deletion
                diagnostics.push(Diagnostic::warning(
                    "Could not check VM status",
                    format!(
                        "Failed to check if VM is running: {}. Attempting deletion anyway.",
                        e
                    ),
                ));
            }
        }

        // Now attempt to delete the VM
        match qemu_api.delete(vmid, false).await {
            Ok(_) => DeleteResourceResponse { diagnostics },
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to delete VM",
                    format!("API error: {}", e),
                ));
                DeleteResourceResponse { diagnostics }
            }
        }
    }
}

impl QemuVmResource {
    fn populate_all_attributes(state: &mut DynamicValue, planned_state: &DynamicValue) {
        // This method ensures ALL schema attributes are present in the state
        // Used when creation fails to avoid "missing attribute" errors

        // Required attributes - preserve from planned state
        if let Ok(node) = planned_state.get_string(&AttributePath::new("target_node")) {
            let _ = state.set_string(&AttributePath::new("target_node"), node);
        }
        if let Ok(vmid) = planned_state.get_number(&AttributePath::new("vmid")) {
            let _ = state.set_number(&AttributePath::new("vmid"), vmid);
        }
        if let Ok(name) = planned_state.get_string(&AttributePath::new("name")) {
            let _ = state.set_string(&AttributePath::new("name"), name);
        }

        // Set all optional attributes to their default values
        // Clone/Template Settings
        let _ = state.set_string(&AttributePath::new("clone"), String::new());
        let _ = state.set_bool(&AttributePath::new("full_clone"), false);
        let _ = state.set_string(&AttributePath::new("os_type"), String::new());

        // Hardware Configuration
        let _ = state.set_string(&AttributePath::new("bios"), "seabios".to_string());
        let _ = state.set_string(&AttributePath::new("machine"), String::new());
        let _ = state.set_string(&AttributePath::new("cpu_type"), String::new());
        let _ = state.set_number(&AttributePath::new("cores"), 1.0);
        let _ = state.set_number(&AttributePath::new("sockets"), 1.0);
        let _ = state.set_number(&AttributePath::new("vcpus"), 0.0);
        let _ = state.set_number(&AttributePath::new("memory"), 512.0);
        let _ = state.set_number(&AttributePath::new("balloon"), 0.0);

        // Boot Configuration
        let _ = state.set_string(&AttributePath::new("boot"), String::new());
        let _ = state.set_string(&AttributePath::new("bootdisk"), String::new());
        let _ = state.set_bool(&AttributePath::new("onboot"), false);

        // Storage Configuration
        let _ = state.set_string(&AttributePath::new("scsihw"), "lsi".to_string());

        // Guest Agent & OS Settings
        let _ = state.set_number(&AttributePath::new("agent"), 0.0);
        let _ = state.set_string(&AttributePath::new("qemu_os"), String::new());

        // Cloud-Init Configuration
        let _ = state.set_string(&AttributePath::new("ipconfig0"), String::new());
        let _ = state.set_string(&AttributePath::new("ipconfig1"), String::new());
        let _ = state.set_string(&AttributePath::new("ipconfig2"), String::new());
        let _ = state.set_string(&AttributePath::new("ipconfig3"), String::new());
        let _ = state.set_string(&AttributePath::new("ciuser"), String::new());
        let _ = state.set_string(&AttributePath::new("cipassword"), String::new());
        let _ = state.set_bool(&AttributePath::new("ciupgrade"), false);
        let _ = state.set_string(&AttributePath::new("sshkeys"), String::new());

        // Network Settings
        let _ = state.set_bool(&AttributePath::new("skip_ipv4"), false);
        let _ = state.set_bool(&AttributePath::new("skip_ipv6"), false);

        // Timing & Behavior Settings
        let _ = state.set_number(&AttributePath::new("additional_wait"), 0.0);
        let _ = state.set_bool(&AttributePath::new("automatic_reboot"), true);
        let _ = state.set_number(&AttributePath::new("clone_wait"), 0.0);
        let _ = state.set_bool(&AttributePath::new("define_connection_info"), false);

        // Other attributes
        let _ = state.set_string(&AttributePath::new("description"), String::new());
        let _ = state.set_bool(&AttributePath::new("start"), false);
        let _ = state.set_bool(&AttributePath::new("tablet"), true);
        let _ = state.set_bool(&AttributePath::new("protection"), false);
        let _ = state.set_string(&AttributePath::new("tags"), String::new());

        // Nested blocks - empty lists with proper structure
        let _ = state.set_list(&AttributePath::new("network"), Vec::new());
        let _ = state.set_list(&AttributePath::new("disk"), Vec::new());
        let _ = state.set_list(&AttributePath::new("cdrom"), Vec::new());
        let _ = state.set_list(&AttributePath::new("cloudinit_drive"), Vec::new());
        let _ = state.set_list(&AttributePath::new("serial"), Vec::new());
        let _ = state.set_list(&AttributePath::new("efidisk"), Vec::new());

        // Now override with any values from planned state
        if let Ok(tags) = planned_state.get_string(&AttributePath::new("tags")) {
            let _ = state.set_string(&AttributePath::new("tags"), tags);
        }
        if let Ok(cores) = planned_state.get_number(&AttributePath::new("cores")) {
            let _ = state.set_number(&AttributePath::new("cores"), cores);
        }
        if let Ok(memory) = planned_state.get_number(&AttributePath::new("memory")) {
            let _ = state.set_number(&AttributePath::new("memory"), memory);
        }
        if let Ok(start) = planned_state.get_bool(&AttributePath::new("start")) {
            let _ = state.set_bool(&AttributePath::new("start"), start);
        }
        // Copy all block values from planned state
        if let Ok(network) = planned_state.get_list(&AttributePath::new("network")) {
            let _ = state.set_list(&AttributePath::new("network"), network);
        }
        if let Ok(disk) = planned_state.get_list(&AttributePath::new("disk")) {
            let _ = state.set_list(&AttributePath::new("disk"), disk);
        }
        if let Ok(cdrom) = planned_state.get_list(&AttributePath::new("cdrom")) {
            let _ = state.set_list(&AttributePath::new("cdrom"), cdrom);
        }
        if let Ok(cloudinit_drive) = planned_state.get_list(&AttributePath::new("cloudinit_drive"))
        {
            let _ = state.set_list(&AttributePath::new("cloudinit_drive"), cloudinit_drive);
        }
        if let Ok(serial) = planned_state.get_list(&AttributePath::new("serial")) {
            let _ = state.set_list(&AttributePath::new("serial"), serial);
        }
        if let Ok(efidisk) = planned_state.get_list(&AttributePath::new("efidisk")) {
            let _ = state.set_list(&AttributePath::new("efidisk"), efidisk);
        }
    }

    fn populate_state_from_config(
        state: &mut DynamicValue,
        vm_config: &crate::api::nodes::QemuConfig,
        planned_state: &DynamicValue,
    ) {
        // Required fields should always be present
        if let Some(name) = &vm_config.name {
            let _ = state.set_string(&AttributePath::new("name"), name.clone());
        }

        // Only populate optional attributes if they exist in VM config or planned state
        if let Some(cores) = vm_config.cores {
            let _ = state.set_number(&AttributePath::new("cores"), cores as f64);
        } else if planned_state
            .get_number(&AttributePath::new("cores"))
            .is_ok()
        {
            let _ = state.set_number(&AttributePath::new("cores"), 1.0);
        }

        if let Some(sockets) = vm_config.sockets {
            let _ = state.set_number(&AttributePath::new("sockets"), sockets as f64);
        } else if planned_state
            .get_number(&AttributePath::new("sockets"))
            .is_ok()
        {
            let _ = state.set_number(&AttributePath::new("sockets"), 1.0);
        }

        if let Some(memory) = vm_config.memory {
            let _ = state.set_number(&AttributePath::new("memory"), memory as f64);
        } else if planned_state
            .get_number(&AttributePath::new("memory"))
            .is_ok()
        {
            let _ = state.set_number(&AttributePath::new("memory"), 512.0);
        }

        if let Some(ref cpu) = vm_config.cpu {
            let _ = state.set_string(&AttributePath::new("cpu"), cpu.clone());
        } else if planned_state.get_string(&AttributePath::new("cpu")).is_ok() {
            let _ = state.set_string(&AttributePath::new("cpu"), "x86-64-v2-AES".to_string());
        }

        if let Some(ref bios) = vm_config.bios {
            let _ = state.set_string(&AttributePath::new("bios"), bios.clone());
        } else if planned_state
            .get_string(&AttributePath::new("bios"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("bios"), "seabios".to_string());
        }

        if let Some(ref boot) = vm_config.boot {
            // Only set if it was also in planned state
            if planned_state
                .get_string(&AttributePath::new("boot"))
                .is_ok()
            {
                let _ = state.set_string(&AttributePath::new("boot"), boot.clone());
            }
        } else if planned_state
            .get_string(&AttributePath::new("boot"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("boot"), String::new());
        }

        if let Some(ref scsihw) = vm_config.scsihw {
            let _ = state.set_string(&AttributePath::new("scsihw"), scsihw.clone());
        } else if planned_state
            .get_string(&AttributePath::new("scsihw"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("scsihw"), "lsi".to_string());
        }

        if let Some(ref ostype) = vm_config.ostype {
            let _ = state.set_string(&AttributePath::new("ostype"), ostype.clone());
        } else if planned_state
            .get_string(&AttributePath::new("ostype"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("ostype"), "other".to_string());
        }

        if let Some(ref agent) = vm_config.agent {
            let _ = state.set_string(&AttributePath::new("agent"), agent.clone());
        } else if planned_state
            .get_string(&AttributePath::new("agent"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("agent"), "0".to_string());
        }

        if let Some(onboot) = vm_config.onboot {
            let _ = state.set_bool(&AttributePath::new("onboot"), onboot);
        } else if planned_state
            .get_bool(&AttributePath::new("onboot"))
            .is_ok()
        {
            let _ = state.set_bool(&AttributePath::new("onboot"), false);
        }

        if let Some(tablet) = vm_config.tablet {
            let _ = state.set_bool(&AttributePath::new("tablet"), tablet);
        } else if planned_state
            .get_bool(&AttributePath::new("tablet"))
            .is_ok()
        {
            let _ = state.set_bool(&AttributePath::new("tablet"), true);
        }

        if let Some(protection) = vm_config.protection {
            let _ = state.set_bool(&AttributePath::new("protection"), protection);
        } else if planned_state
            .get_bool(&AttributePath::new("protection"))
            .is_ok()
        {
            let _ = state.set_bool(&AttributePath::new("protection"), false);
        }

        if let Some(tags) = &vm_config.tags {
            // Only set if it was also in planned state
            if planned_state
                .get_string(&AttributePath::new("tags"))
                .is_ok()
            {
                let normalized_tags = Self::normalize_tags(tags);
                let _ = state.set_string(&AttributePath::new("tags"), normalized_tags);
            }
        } else if planned_state
            .get_string(&AttributePath::new("tags"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("tags"), String::new());
        }

        if let Some(ref description) = vm_config.description {
            // Only set if it was also in planned state
            if planned_state
                .get_string(&AttributePath::new("description"))
                .is_ok()
            {
                let _ = state.set_string(&AttributePath::new("description"), description.clone());
            }
        } else if planned_state
            .get_string(&AttributePath::new("description"))
            .is_ok()
        {
            let _ = state.set_string(&AttributePath::new("description"), String::new());
        }

        // Disk configurations - only populate if in planned state or VM config
        let disk_attrs = vec![
            ("scsi0", &vm_config.scsi0),
            ("scsi1", &vm_config.scsi1),
            ("scsi2", &vm_config.scsi2),
            ("scsi3", &vm_config.scsi3),
            ("virtio0", &vm_config.virtio0),
            ("virtio1", &vm_config.virtio1),
            ("ide0", &vm_config.ide0),
            ("ide2", &vm_config.ide2),
            ("sata0", &vm_config.sata0),
            ("efidisk0", &vm_config.efidisk0),
        ];

        for (attr_name, disk_config) in disk_attrs {
            if let Some(config) = disk_config {
                let current_config = planned_state
                    .get_string(&AttributePath::new(attr_name))
                    .ok();
                let normalized_disk =
                    Self::normalize_disk_config(config, current_config.as_deref());
                let _ = state.set_string(&AttributePath::new(attr_name), normalized_disk);
            } else if planned_state
                .get_string(&AttributePath::new(attr_name))
                .is_ok()
            {
                // Only set empty string if it was in planned state
                let _ = state.set_string(&AttributePath::new(attr_name), String::new());
            }
        }

        // Network configurations - only populate if in planned state or VM config
        let net_attrs = vec![
            ("net0", &vm_config.net0),
            ("net1", &vm_config.net1),
            ("net2", &vm_config.net2),
            ("net3", &vm_config.net3),
        ];

        for (attr_name, net_config) in net_attrs {
            if let Some(config) = net_config {
                let current_config = planned_state
                    .get_string(&AttributePath::new(attr_name))
                    .ok();
                let normalized_net =
                    Self::normalize_network_config(config, current_config.as_deref());
                let _ = state.set_string(&AttributePath::new(attr_name), normalized_net);
            } else if planned_state
                .get_string(&AttributePath::new(attr_name))
                .is_ok()
            {
                // Only set empty string if it was in planned state
                let _ = state.set_string(&AttributePath::new(attr_name), String::new());
            }
        }

        // Cloud-init attributes - only set if present in planned state
        if let Ok(ciuser) = planned_state.get_string(&AttributePath::new("ciuser")) {
            let _ = state.set_string(&AttributePath::new("ciuser"), ciuser);
        }

        if let Ok(cipassword) = planned_state.get_string(&AttributePath::new("cipassword")) {
            let _ = state.set_string(&AttributePath::new("cipassword"), cipassword);
        }

        if let Ok(sshkeys) = planned_state.get_string(&AttributePath::new("sshkeys")) {
            let _ = state.set_string(&AttributePath::new("sshkeys"), sshkeys);
        }

        if let Ok(ipconfig0) = planned_state.get_string(&AttributePath::new("ipconfig0")) {
            let _ = state.set_string(&AttributePath::new("ipconfig0"), ipconfig0);
        }

        if let Ok(ipconfig1) = planned_state.get_string(&AttributePath::new("ipconfig1")) {
            let _ = state.set_string(&AttributePath::new("ipconfig1"), ipconfig1);
        }

        if let Ok(ipconfig2) = planned_state.get_string(&AttributePath::new("ipconfig2")) {
            let _ = state.set_string(&AttributePath::new("ipconfig2"), ipconfig2);
        }

        if let Ok(ipconfig3) = planned_state.get_string(&AttributePath::new("ipconfig3")) {
            let _ = state.set_string(&AttributePath::new("ipconfig3"), ipconfig3);
        }

        // Start attribute - preserve from planned state
        if let Ok(start) = planned_state.get_bool(&AttributePath::new("start")) {
            let _ = state.set_bool(&AttributePath::new("start"), start);
        }
    }

    fn populate_state_with_nested_blocks(
        state: &mut DynamicValue,
        vm_config: &crate::api::nodes::QemuConfig,
        planned_state: &DynamicValue,
    ) {
        // First populate all the basic fields
        Self::populate_state_from_config(state, vm_config, planned_state);

        // Handle network blocks
        let mut networks = Vec::new();

        // Check if we have network blocks in planned state
        if let Ok(planned_networks) = planned_state.get_list(&AttributePath::new("network")) {
            // Only convert networks that were in planned blocks
            let mut planned_network_ids = std::collections::HashSet::new();
            for net in &planned_networks {
                if let Dynamic::Map(net_map) = net {
                    if let Some(Dynamic::Number(id)) = net_map.get("id") {
                        planned_network_ids.insert(*id as u32);
                    }
                }
            }

            // Build network blocks from VM config
            for i in 0..=3 {
                // Only include networks that were in the planned blocks
                if !planned_network_ids.contains(&i) {
                    continue;
                }

                let net_field = match i {
                    0 => &vm_config.net0,
                    1 => &vm_config.net1,
                    2 => &vm_config.net2,
                    3 => &vm_config.net3,
                    _ => &None,
                };

                if let Some(net_config) = net_field {
                    // Parse the network string and create a block
                    let net_block = Self::parse_network_string(net_config, i);
                    networks.push(net_block);
                }
            }

            // Always set the list, even if empty
            let _ = state.set_list(&AttributePath::new("network"), networks);
        }

        // Handle disk blocks
        let mut disks = Vec::new();

        // Check if we have disk blocks in planned state
        if let Ok(planned_disks) = planned_state.get_list(&AttributePath::new("disk")) {
            // Only convert disks that were in planned blocks
            let mut planned_disk_slots = std::collections::HashSet::new();
            for disk in &planned_disks {
                if let Dynamic::Map(disk_map) = disk {
                    if let Some(Dynamic::String(slot)) = disk_map.get("slot") {
                        planned_disk_slots.insert(slot.clone());
                    }
                }
            }

            // Build disk blocks from VM config
            let disk_configs = vec![
                ("scsi0", &vm_config.scsi0),
                ("scsi1", &vm_config.scsi1),
                ("scsi2", &vm_config.scsi2),
                ("scsi3", &vm_config.scsi3),
                ("virtio0", &vm_config.virtio0),
                ("virtio1", &vm_config.virtio1),
                ("ide0", &vm_config.ide0),
                ("ide2", &vm_config.ide2),
                ("sata0", &vm_config.sata0),
            ];

            for (slot, disk_field) in disk_configs {
                // Only include disks that were in the planned blocks
                if !planned_disk_slots.contains(slot) {
                    continue;
                }

                if let Some(disk_config) = disk_field {
                    // Parse the disk string and create a block
                    let disk_block = Self::parse_disk_string(disk_config, slot);
                    disks.push(disk_block);
                }
            }

            // Always set the list, even if empty
            let _ = state.set_list(&AttributePath::new("disk"), disks);
        }

        // Handle efidisk block (it's a list with max_items: 1)
        if let Ok(efidisk_list) = planned_state.get_list(&AttributePath::new("efidisk")) {
            if !efidisk_list.is_empty() {
                let mut efidisk_blocks = vec![];
                let mut efidisk = std::collections::HashMap::new();

                if let Some(efidisk_config) = &vm_config.efidisk0 {
                    // Parse storage and format from config like "local-lvm:1,format=raw,efitype=4m"
                    let parts: Vec<&str> = efidisk_config.split(',').collect();
                    if let Some(storage_part) = parts.first() {
                        if let Some((storage, _)) = storage_part.split_once(':') {
                            efidisk.insert(
                                "storage".to_string(),
                                Dynamic::String(storage.to_string()),
                            );
                        }
                    }

                    for part in parts.iter().skip(1) {
                        if let Some((key, value)) = part.split_once('=') {
                            match key {
                                "format" => {
                                    efidisk.insert(
                                        "format".to_string(),
                                        Dynamic::String(value.to_string()),
                                    );
                                }
                                "efitype" => {
                                    efidisk.insert(
                                        "efitype".to_string(),
                                        Dynamic::String(value.to_string()),
                                    );
                                }
                                "pre-enrolled-keys" => {
                                    let enrolled = value == "1" || value == "true";
                                    efidisk.insert(
                                        "pre_enrolled_keys".to_string(),
                                        Dynamic::Bool(enrolled),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Copy all values from planned state first
                if let Some(Dynamic::Map(planned_map)) = efidisk_list.first() {
                    // Start with all planned values
                    for (key, value) in planned_map {
                        if !efidisk.contains_key(key) {
                            efidisk.insert(key.clone(), value.clone());
                        }
                    }
                }

                // Ensure all required attributes are present with defaults if not in API response
                if !efidisk.contains_key("storage") {
                    efidisk.insert("storage".to_string(), Dynamic::String(String::new()));
                }
                if !efidisk.contains_key("format") {
                    efidisk.insert("format".to_string(), Dynamic::String("raw".to_string()));
                }
                if !efidisk.contains_key("efitype") {
                    efidisk.insert("efitype".to_string(), Dynamic::String("4m".to_string()));
                }
                if !efidisk.contains_key("pre_enrolled_keys") {
                    efidisk.insert("pre_enrolled_keys".to_string(), Dynamic::Bool(false));
                }

                // Always set the map
                efidisk_blocks.push(Dynamic::Map(efidisk));
                let _ = state.set_list(&AttributePath::new("efidisk"), efidisk_blocks);
            }
        }

        // Handle cloudinit block
        if planned_state
            .get_map(&AttributePath::new("cloudinit"))
            .is_ok()
        {
            let mut cloudinit = std::collections::HashMap::new();

            // Get cloud-init values from planned state since Proxmox doesn't return them
            if let Ok(ci_map) = planned_state.get_map(&AttributePath::new("cloudinit")) {
                // Copy all values from planned state
                for (key, value) in ci_map {
                    cloudinit.insert(key, value);
                }
            }

            // Ensure all required attributes are present
            if !cloudinit.contains_key("user") {
                cloudinit.insert("user".to_string(), Dynamic::String(String::new()));
            }
            if !cloudinit.contains_key("password") {
                cloudinit.insert("password".to_string(), Dynamic::String(String::new()));
            }
            if !cloudinit.contains_key("ssh_keys") {
                cloudinit.insert("ssh_keys".to_string(), Dynamic::String(String::new()));
            }
            if !cloudinit.contains_key("ipconfig") {
                cloudinit.insert("ipconfig".to_string(), Dynamic::List(Vec::new()));
            }

            let _ = state.set_map(&AttributePath::new("cloudinit"), cloudinit);
        }
    }

    fn extract_vm_config(
        &self,
        config: &DynamicValue,
    ) -> Result<(String, u32, crate::api::nodes::CreateQemuRequest), Diagnostic> {
        // Core VM Identity - note: changed from "node" to "target_node"
        let node = config
            .get_string(&AttributePath::new("target_node"))
            .map_err(|_| {
                Diagnostic::error(
                    "Missing target_node",
                    "The 'target_node' attribute is required",
                )
            })?;

        let vmid = config
            .get_number(&AttributePath::new("vmid"))
            .map_err(|_| Diagnostic::error("Missing vmid", "The 'vmid' attribute is required"))?
            as u32;

        let name = config.get_string(&AttributePath::new("name")).ok();
        let tags = config.get_string(&AttributePath::new("tags")).ok();

        // Clone/Template Settings
        let clone = config.get_string(&AttributePath::new("clone")).ok();
        let full_clone = config.get_bool(&AttributePath::new("full_clone")).ok();
        let os_type = config.get_string(&AttributePath::new("os_type")).ok();

        // Hardware Configuration
        let bios = config.get_string(&AttributePath::new("bios")).ok();
        let machine = config.get_string(&AttributePath::new("machine")).ok();
        let cpu_type = config.get_string(&AttributePath::new("cpu_type")).ok();
        let cores = config
            .get_number(&AttributePath::new("cores"))
            .ok()
            .map(|n| n as u32);
        let sockets = config
            .get_number(&AttributePath::new("sockets"))
            .ok()
            .map(|n| n as u32);
        let vcpus = config
            .get_number(&AttributePath::new("vcpus"))
            .ok()
            .map(|n| n as u32);
        let memory = config
            .get_number(&AttributePath::new("memory"))
            .ok()
            .map(|n| n as u64);
        let balloon = config
            .get_number(&AttributePath::new("balloon"))
            .ok()
            .map(|n| n as u64);

        // Boot Configuration
        let boot = config.get_string(&AttributePath::new("boot")).ok();
        let bootdisk = config.get_string(&AttributePath::new("bootdisk")).ok();
        let onboot = config.get_bool(&AttributePath::new("onboot")).ok();

        // Storage Configuration
        let scsihw = config.get_string(&AttributePath::new("scsihw")).ok();

        // Guest Agent & OS Settings
        let agent = config
            .get_number(&AttributePath::new("agent"))
            .ok()
            .map(|n| n.to_string());
        let qemu_os = config.get_string(&AttributePath::new("qemu_os")).ok();

        // Cloud-Init Configuration
        let ipconfig0 = config.get_string(&AttributePath::new("ipconfig0")).ok();
        let ipconfig1 = config.get_string(&AttributePath::new("ipconfig1")).ok();
        let ciuser = config.get_string(&AttributePath::new("ciuser")).ok();
        let cipassword = config.get_string(&AttributePath::new("cipassword")).ok();
        let ciupgrade = config.get_bool(&AttributePath::new("ciupgrade")).ok();
        let sshkeys = config.get_string(&AttributePath::new("sshkeys")).ok();

        // Other attributes
        let start = config.get_bool(&AttributePath::new("start")).ok();
        let tablet = config.get_bool(&AttributePath::new("tablet")).ok();
        let protection = config.get_bool(&AttributePath::new("protection")).ok();
        let description = config.get_string(&AttributePath::new("description")).ok();

        // Handle disk blocks
        let mut scsi0 = None;
        let mut scsi1 = None;
        let mut scsi2 = None;
        let mut scsi3 = None;
        let mut virtio0 = None;
        let mut virtio1 = None;
        let mut ide0 = None;
        let mut ide2 = None;
        let mut ide3 = None;
        let mut sata0 = None;

        // Process disk blocks
        if let Ok(disks) = config.get_list(&AttributePath::new("disk")) {
            for disk in disks {
                if let Ok((slot, disk_string)) = Self::disk_block_to_api_string(&disk) {
                    match slot.as_str() {
                        "scsi0" => scsi0 = Some(disk_string),
                        "scsi1" => scsi1 = Some(disk_string),
                        "scsi2" => scsi2 = Some(disk_string),
                        "scsi3" => scsi3 = Some(disk_string),
                        "virtio0" => virtio0 = Some(disk_string),
                        "virtio1" => virtio1 = Some(disk_string),
                        "ide0" => ide0 = Some(disk_string),
                        "ide2" => ide2 = Some(disk_string),
                        "ide3" => ide3 = Some(disk_string),
                        "sata0" => sata0 = Some(disk_string),
                        _ => {} // Ignore other slots
                    }
                }
            }
        }

        // Process cdrom blocks
        if let Ok(cdroms) = config.get_list(&AttributePath::new("cdrom")) {
            for cdrom in cdroms {
                if let Ok((slot, cdrom_string)) = Self::cdrom_block_to_api_string(&cdrom) {
                    if slot.as_str() == "ide2" {
                        ide2 = Some(cdrom_string);
                    }
                }
            }
        }

        // Process cloudinit_drive blocks
        if let Ok(cloudinit_drives) = config.get_list(&AttributePath::new("cloudinit_drive")) {
            for ci_drive in cloudinit_drives {
                if let Ok((slot, ci_string)) = Self::cloudinit_drive_block_to_api_string(&ci_drive)
                {
                    if slot.as_str() == "ide3" {
                        ide3 = Some(ci_string);
                    }
                }
            }
        }

        // Handle efidisk
        let mut efidisk0 = None;
        if let Ok(efidisks) = config.get_list(&AttributePath::new("efidisk")) {
            if let Some(efidisk) = efidisks.first() {
                if let Ok(efidisk_string) = Self::efidisk_block_to_api_string(efidisk) {
                    efidisk0 = Some(efidisk_string);
                }
            }
        }

        // Handle serial blocks
        let mut serial0 = None;
        let mut serial1 = None;
        let mut serial2 = None;
        let mut serial3 = None;
        if let Ok(serials) = config.get_list(&AttributePath::new("serial")) {
            for serial in serials {
                if let Ok((id, serial_string)) = Self::serial_block_to_api_string(&serial) {
                    match id {
                        0 => serial0 = Some(serial_string),
                        1 => serial1 = Some(serial_string),
                        2 => serial2 = Some(serial_string),
                        3 => serial3 = Some(serial_string),
                        _ => {} // Ignore other IDs
                    }
                }
            }
        }

        // Handle networks - check for nested blocks first, then fall back to string attributes
        let mut net0 = None;
        let mut net1 = None;
        let mut net2 = None;
        let mut net3 = None;

        // Check for network blocks
        if let Ok(networks) = config.get_list(&AttributePath::new("network")) {
            for net in networks {
                if let Dynamic::Map(ref net_map) = net {
                    if let Some(Dynamic::Number(id)) = net_map.get("id") {
                        let id_int = *id as u32;
                        if let Ok(net_string) = Self::network_blocks_to_string(&[net]) {
                            match id_int {
                                0 => net0 = Some(net_string),
                                1 => net1 = Some(net_string),
                                2 => net2 = Some(net_string),
                                3 => net3 = Some(net_string),
                                _ => {} // Ignore IDs > 3 for now
                            }
                        }
                    }
                }
            }
        }

        // Fall back to string attributes if no network blocks
        if net0.is_none() {
            net0 = config
                .get_string(&AttributePath::new("net0"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }
        if net1.is_none() {
            net1 = config
                .get_string(&AttributePath::new("net1"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }
        if net2.is_none() {
            net2 = config
                .get_string(&AttributePath::new("net2"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }
        if net3.is_none() {
            net3 = config
                .get_string(&AttributePath::new("net3"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }

        let create_request = crate::api::nodes::CreateQemuRequest {
            vmid,
            clone: clone.clone(),
            full: if clone.is_some() { full_clone } else { None },
            name,
            cores,
            sockets,
            memory,
            cpu: cpu_type,
            bios,
            boot,
            bootdisk,
            scsihw,
            ostype: qemu_os.clone().or(os_type),
            agent,
            onboot,
            start,
            tablet,
            protection,
            tags,
            description,
            scsi0,
            scsi1,
            scsi2,
            scsi3,
            virtio0,
            virtio1,
            ide0,
            ide2,
            ide3,
            sata0,
            net0,
            net1,
            net2,
            net3,
            acpi: None,
            args: None,
            autostart: None,
            balloon,
            cdrom: None,
            cpulimit: None,
            cpuunits: None,
            efidisk0,
            freeze: None,
            hookscript: None,
            hotplug: None,
            hugepages: None,
            ide1: None,
            kvm: None,
            localtime: None,
            lock: None,
            machine,
            migrate_downtime: None,
            migrate_speed: None,
            nameserver: None,
            numa: None,
            numa0: None,
            numa1: None,
            reboot: None,
            sata1: None,
            sata2: None,
            sata3: None,
            sata4: None,
            sata5: None,
            scsi4: None,
            scsi5: None,
            scsi6: None,
            scsi7: None,
            searchdomain: None,
            serial0,
            serial1,
            serial2,
            serial3,
            shares: None,
            smbios1: None,
            smp: None,
            startup: None,
            startdate: None,
            template: None,
            unused0: None,
            unused1: None,
            unused2: None,
            unused3: None,
            usb0: None,
            usb1: None,
            usb2: None,
            usb3: None,
            vcpus,
            vga: None,
            virtio2: None,
            virtio3: None,
            virtio4: None,
            virtio5: None,
            virtio6: None,
            virtio7: None,
            virtio8: None,
            virtio9: None,
            virtio10: None,
            virtio11: None,
            virtio12: None,
            virtio13: None,
            virtio14: None,
            virtio15: None,
            vmgenid: None,
            vmstatestorage: None,
            watchdog: None,
            ciuser,
            cipassword,
            ciupgrade,
            ipconfig0,
            ipconfig1,
            sshkeys,
        };

        Ok((node, vmid, create_request))
    }

    fn build_update_request(
        &self,
        config: &DynamicValue,
    ) -> Result<crate::api::nodes::UpdateQemuRequest, Diagnostic> {
        let name = config.get_string(&AttributePath::new("name")).ok();
        let cores = config
            .get_number(&AttributePath::new("cores"))
            .ok()
            .map(|n| n as u32);
        let sockets = config
            .get_number(&AttributePath::new("sockets"))
            .ok()
            .map(|n| n as u32);
        let memory = config
            .get_number(&AttributePath::new("memory"))
            .ok()
            .map(|n| n as u64);
        let cpu = config.get_string(&AttributePath::new("cpu")).ok();
        let bios = config.get_string(&AttributePath::new("bios")).ok();
        let boot = config.get_string(&AttributePath::new("boot")).ok();
        let scsihw = config.get_string(&AttributePath::new("scsihw")).ok();
        let ostype = config.get_string(&AttributePath::new("ostype")).ok();
        let agent = config.get_string(&AttributePath::new("agent")).ok();
        let onboot = config.get_bool(&AttributePath::new("onboot")).ok();
        let tablet = config.get_bool(&AttributePath::new("tablet")).ok();
        let protection = config.get_bool(&AttributePath::new("protection")).ok();
        let tags = config.get_string(&AttributePath::new("tags")).ok();
        let description = config.get_string(&AttributePath::new("description")).ok();

        // Handle disks - check for nested blocks first, then fall back to string attributes
        let mut scsi0 = None;
        let mut scsi1 = None;
        let mut scsi2 = None;
        let mut scsi3 = None;
        let mut virtio0 = None;
        let mut virtio1 = None;
        let mut ide0 = None;
        let mut ide2 = None;
        let mut sata0 = None;

        // Check for disk blocks
        if let Ok(disks) = config.get_list(&AttributePath::new("disk")) {
            for disk in disks {
                if let Ok((slot, disk_string)) = Self::disk_block_to_api_string(&disk) {
                    match slot.as_str() {
                        "scsi0" => scsi0 = Some(disk_string),
                        "scsi1" => scsi1 = Some(disk_string),
                        "scsi2" => scsi2 = Some(disk_string),
                        "scsi3" => scsi3 = Some(disk_string),
                        "virtio0" => virtio0 = Some(disk_string),
                        "virtio1" => virtio1 = Some(disk_string),
                        "ide0" => ide0 = Some(disk_string),
                        "ide2" => ide2 = Some(disk_string),
                        "sata0" => sata0 = Some(disk_string),
                        _ => {} // Ignore other interfaces for now
                    }
                }
            }
        }

        // Fall back to string attributes if no disk blocks
        if scsi0.is_none() {
            scsi0 = config.get_string(&AttributePath::new("scsi0")).ok();
        }
        if scsi1.is_none() {
            scsi1 = config.get_string(&AttributePath::new("scsi1")).ok();
        }
        if scsi2.is_none() {
            scsi2 = config.get_string(&AttributePath::new("scsi2")).ok();
        }
        if scsi3.is_none() {
            scsi3 = config.get_string(&AttributePath::new("scsi3")).ok();
        }
        if virtio0.is_none() {
            virtio0 = config.get_string(&AttributePath::new("virtio0")).ok();
        }
        if virtio1.is_none() {
            virtio1 = config.get_string(&AttributePath::new("virtio1")).ok();
        }
        if ide0.is_none() {
            ide0 = config.get_string(&AttributePath::new("ide0")).ok();
        }
        if ide2.is_none() {
            ide2 = config.get_string(&AttributePath::new("ide2")).ok();
        }
        if sata0.is_none() {
            sata0 = config.get_string(&AttributePath::new("sata0")).ok();
        }

        // Handle efidisk - check for nested block first (it's a list), then fall back to string attribute
        let mut efidisk0 = None;
        if let Ok(efidisks) = config.get_list(&AttributePath::new("efidisk")) {
            if let Some(efidisk) = efidisks.first() {
                if let Ok(efidisk_string) = Self::efidisk_block_to_api_string(efidisk) {
                    efidisk0 = Some(efidisk_string);
                }
            }
        }
        if efidisk0.is_none() {
            efidisk0 = config.get_string(&AttributePath::new("efidisk0")).ok();
        }

        // Handle networks - check for nested blocks first, then fall back to string attributes
        let mut net0 = None;
        let mut net1 = None;
        let mut net2 = None;
        let mut net3 = None;

        // Check for network blocks
        if let Ok(networks) = config.get_list(&AttributePath::new("network")) {
            for net in networks {
                if let Dynamic::Map(ref net_map) = net {
                    if let Some(Dynamic::Number(id)) = net_map.get("id") {
                        let id_int = *id as u32;
                        if let Ok(net_string) = Self::network_blocks_to_string(&[net]) {
                            match id_int {
                                0 => net0 = Some(net_string),
                                1 => net1 = Some(net_string),
                                2 => net2 = Some(net_string),
                                3 => net3 = Some(net_string),
                                _ => {} // Ignore IDs > 3 for now
                            }
                        }
                    }
                }
            }
        }

        // Fall back to string attributes if no network blocks
        if net0.is_none() {
            net0 = config
                .get_string(&AttributePath::new("net0"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }
        if net1.is_none() {
            net1 = config
                .get_string(&AttributePath::new("net1"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }
        if net2.is_none() {
            net2 = config
                .get_string(&AttributePath::new("net2"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }
        if net3.is_none() {
            net3 = config
                .get_string(&AttributePath::new("net3"))
                .ok()
                .map(|n| Self::normalize_network_config(&n, Some(&n)));
        }

        Ok(crate::api::nodes::UpdateQemuRequest {
            name,
            cores,
            sockets,
            memory,
            cpu,
            bios,
            boot,
            scsihw,
            ostype,
            agent,
            onboot,
            tablet,
            protection,
            tags,
            description,
            scsi0,
            scsi1,
            scsi2,
            scsi3,
            virtio0,
            virtio1,
            ide0,
            ide2,
            sata0,
            net0,
            net1,
            net2,
            net3,
            acpi: None,
            args: None,
            autostart: None,
            balloon: None,
            bootdisk: None,
            cdrom: None,
            cpulimit: None,
            cpuunits: None,
            delete: None,
            digest: None,
            efidisk0,
            freeze: None,
            hookscript: None,
            hotplug: None,
            hugepages: None,
            ide1: None,
            ide3: None,
            kvm: None,
            localtime: None,
            lock: None,
            machine: None,
            migrate_downtime: None,
            migrate_speed: None,
            nameserver: None,
            numa: None,
            numa0: None,
            numa1: None,
            reboot: None,
            revert: None,
            sata1: None,
            sata2: None,
            sata3: None,
            sata4: None,
            sata5: None,
            scsi4: None,
            scsi5: None,
            scsi6: None,
            scsi7: None,
            searchdomain: None,
            serial0: None,
            serial1: None,
            serial2: None,
            serial3: None,
            shares: None,
            smbios1: None,
            smp: None,
            startup: None,
            startdate: None,
            template: None,
            unused0: None,
            unused1: None,
            unused2: None,
            unused3: None,
            usb0: None,
            usb1: None,
            usb2: None,
            usb3: None,
            vcpus: None,
            vga: None,
            virtio2: None,
            virtio3: None,
            virtio4: None,
            virtio5: None,
            virtio6: None,
            virtio7: None,
            virtio8: None,
            virtio9: None,
            virtio10: None,
            virtio11: None,
            virtio12: None,
            virtio13: None,
            virtio14: None,
            virtio15: None,
            vmgenid: None,
            vmstatestorage: None,
            watchdog: None,
        })
    }
}

#[async_trait]
impl ResourceWithConfigure for QemuVmResource {
    async fn configure(
        &mut self,
        _ctx: Context,
        request: ConfigureResourceRequest,
    ) -> ConfigureResourceResponse {
        let mut diagnostics = vec![];

        if let Some(data) = request.provider_data {
            if let Some(provider_data) = data.downcast_ref::<crate::ProxmoxProviderData>() {
                self.provider_data = Some(provider_data.clone());
            } else {
                diagnostics.push(Diagnostic::error(
                    "Invalid provider data",
                    "Failed to extract ProxmoxProviderData from provider data",
                ));
            }
        } else {
            diagnostics.push(Diagnostic::error(
                "No provider data",
                "No provider data was provided to the resource",
            ));
        }

        ConfigureResourceResponse { diagnostics }
    }
}

#[async_trait]
impl ResourceWithImportState for QemuVmResource {
    async fn import_state(
        &self,
        _ctx: Context,
        request: ImportResourceStateRequest,
    ) -> ImportResourceStateResponse {
        let mut diagnostics = vec![];
        let parts: Vec<&str> = request.id.split('/').collect();

        if parts.len() != 2 {
            diagnostics.push(Diagnostic::error(
                "Invalid import ID",
                "Import ID must be in the format 'node/vmid'",
            ));
            return ImportResourceStateResponse {
                imported_resources: vec![],
                diagnostics,
                deferred: None,
            };
        }

        let node = parts[0];
        let vmid_str = parts[1];

        let vmid = match vmid_str.parse::<u32>() {
            Ok(vmid) => vmid,
            Err(_) => {
                diagnostics.push(Diagnostic::error(
                    "Invalid VMID",
                    "VMID must be a valid number",
                ));
                return ImportResourceStateResponse {
                    imported_resources: vec![],
                    diagnostics,
                    deferred: None,
                };
            }
        };

        // Fetch the VM configuration from the API
        let provider_data = match &self.provider_data {
            Some(data) => data,
            None => {
                diagnostics.push(Diagnostic::error(
                    "Provider not configured",
                    "Unable to import resource without provider configuration",
                ));
                return ImportResourceStateResponse {
                    imported_resources: vec![],
                    diagnostics,
                    deferred: None,
                };
            }
        };

        let config = match provider_data
            .client
            .nodes()
            .node(node)
            .qemu()
            .get_config(vmid)
            .await
        {
            Ok(config) => config,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    "Failed to fetch VM configuration",
                    format!("Error fetching VM {}: {}", vmid, e),
                ));
                return ImportResourceStateResponse {
                    imported_resources: vec![],
                    diagnostics,
                    deferred: None,
                };
            }
        };

        // Build state from the fetched configuration
        let mut state = DynamicValue::new(Dynamic::Map(HashMap::new()));
        let _ = state.set_string(&AttributePath::new("target_node"), node.to_string());
        let _ = state.set_number(&AttributePath::new("vmid"), vmid as f64);

        if let Some(name) = &config.name {
            let _ = state.set_string(&AttributePath::new("name"), name.clone());
        }
        if let Some(cores) = config.cores {
            let _ = state.set_number(&AttributePath::new("cores"), cores as f64);
        }
        if let Some(memory) = config.memory {
            let _ = state.set_number(&AttributePath::new("memory"), memory as f64);
        }
        if let Some(sockets) = config.sockets {
            let _ = state.set_number(&AttributePath::new("sockets"), sockets as f64);
        }
        if let Some(cpu) = &config.cpu {
            let _ = state.set_string(&AttributePath::new("cpu"), cpu.clone());
        }
        if let Some(bios) = &config.bios {
            let _ = state.set_string(&AttributePath::new("bios"), bios.clone());
        }
        if let Some(ostype) = &config.ostype {
            let _ = state.set_string(&AttributePath::new("ostype"), ostype.clone());
        }
        if let Some(description) = &config.description {
            let _ = state.set_string(&AttributePath::new("description"), description.clone());
        }
        if let Some(efidisk0) = &config.efidisk0 {
            let _ = state.set_string(&AttributePath::new("efidisk0"), efidisk0.clone());
        }

        ImportResourceStateResponse {
            imported_resources: vec![ImportedResource {
                type_name: self.type_name().to_string(),
                state,
                private: vec![],
                identity: None,
            }],
            diagnostics,
            deferred: None,
        }
    }
}

#[cfg(test)]
#[path = "./resource_vm_test.rs"]
mod resource_vm_test;
