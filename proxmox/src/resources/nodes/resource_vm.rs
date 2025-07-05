//! QEMU VM resource implementation

use async_trait::async_trait;
use std::collections::HashMap;
use tfplug::context::Context;
use tfplug::resource::{
    ConfigureResourceRequest, ConfigureResourceResponse, CreateResourceRequest,
    CreateResourceResponse, DeleteResourceRequest, DeleteResourceResponse,
    ImportResourceStateRequest, ImportResourceStateResponse, ImportedResource, ReadResourceRequest,
    ReadResourceResponse, Resource, ResourceMetadataRequest, ResourceMetadataResponse,
    ResourceSchemaRequest, ResourceSchemaResponse, ResourceWithConfigure, ResourceWithImportState,
    UpdateResourceRequest, UpdateResourceResponse, ValidateResourceConfigRequest,
    ValidateResourceConfigResponse,
};
use tfplug::schema::{AttributeBuilder, AttributeType, SchemaBuilder};
use tfplug::types::{AttributePath, Diagnostic, Dynamic, DynamicValue};

#[derive(Default)]
pub struct QemuVmResource {
    provider_data: Option<crate::ProxmoxProviderData>,
}

impl QemuVmResource {
    pub fn new() -> Self {
        Self::default()
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
            .attribute(
                AttributeBuilder::new("node", AttributeType::String)
                    .description("The name of the Proxmox node where the VM will be created")
                    .required()
                    .build(),
            )
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
                AttributeBuilder::new("memory", AttributeType::Number)
                    .description("Memory size in MB")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("cpu", AttributeType::String)
                    .description("CPU type (e.g., x86-64-v2-AES, host)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("bios", AttributeType::String)
                    .description("BIOS type (seabios or ovmf)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("boot", AttributeType::String)
                    .description("Boot order and options")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("scsihw", AttributeType::String)
                    .description("SCSI controller model")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ostype", AttributeType::String)
                    .description("Operating system type")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("agent", AttributeType::String)
                    .description("QEMU Guest Agent configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("onboot", AttributeType::Bool)
                    .description("Start VM on boot")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("tablet", AttributeType::Bool)
                    .description("Enable tablet device for mouse")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("protection", AttributeType::Bool)
                    .description("Enable protection against accidental deletion")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("tags", AttributeType::String)
                    .description("VM tags (semicolon separated)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("description", AttributeType::String)
                    .description("VM description")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("scsi0", AttributeType::String)
                    .description("SCSI disk 0 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("scsi1", AttributeType::String)
                    .description("SCSI disk 1 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("scsi2", AttributeType::String)
                    .description("SCSI disk 2 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("scsi3", AttributeType::String)
                    .description("SCSI disk 3 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("virtio0", AttributeType::String)
                    .description("VirtIO disk 0 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("virtio1", AttributeType::String)
                    .description("VirtIO disk 1 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ide0", AttributeType::String)
                    .description("IDE disk 0 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ide2", AttributeType::String)
                    .description("IDE disk 2 configuration (CD-ROM)")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("sata0", AttributeType::String)
                    .description("SATA disk 0 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("net0", AttributeType::String)
                    .description("Network interface 0 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("net1", AttributeType::String)
                    .description("Network interface 1 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("net2", AttributeType::String)
                    .description("Network interface 2 configuration")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("net3", AttributeType::String)
                    .description("Network interface 3 configuration")
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
                    .optional()
                    .sensitive()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("sshkeys", AttributeType::String)
                    .description("Cloud-init SSH public keys")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig0", AttributeType::String)
                    .description("Cloud-init IP configuration for interface 0")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig1", AttributeType::String)
                    .description("Cloud-init IP configuration for interface 1")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig2", AttributeType::String)
                    .description("Cloud-init IP configuration for interface 2")
                    .optional()
                    .build(),
            )
            .attribute(
                AttributeBuilder::new("ipconfig3", AttributeType::String)
                    .description("Cloud-init IP configuration for interface 3")
                    .optional()
                    .build(),
            )
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
        }

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
                    Ok(_task_id) => CreateResourceResponse {
                        new_state: request.planned_state,
                        private: vec![],
                        diagnostics,
                    },
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            "Failed to create VM",
                            format!("API error: {}", e),
                        ));
                        CreateResourceResponse {
                            new_state: request.planned_state,
                            private: vec![],
                            diagnostics,
                        }
                    }
                }
            }
            Err(diag) => {
                diagnostics.push(diag);
                CreateResourceResponse {
                    new_state: request.planned_state,
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
            .get_string(&AttributePath::new("node"))
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

                if let Some(name) = vm_config.name {
                    let _ = new_state.set_string(&AttributePath::new("name"), name);
                }
                if let Some(cores) = vm_config.cores {
                    let _ = new_state.set_number(&AttributePath::new("cores"), cores as f64);
                }
                if let Some(sockets) = vm_config.sockets {
                    let _ = new_state.set_number(&AttributePath::new("sockets"), sockets as f64);
                }
                if let Some(memory) = vm_config.memory {
                    let _ = new_state.set_number(&AttributePath::new("memory"), memory as f64);
                }
                if let Some(cpu) = vm_config.cpu {
                    let _ = new_state.set_string(&AttributePath::new("cpu"), cpu);
                }
                if let Some(bios) = vm_config.bios {
                    let _ = new_state.set_string(&AttributePath::new("bios"), bios);
                }
                if let Some(boot) = vm_config.boot {
                    let _ = new_state.set_string(&AttributePath::new("boot"), boot);
                }
                if let Some(scsihw) = vm_config.scsihw {
                    let _ = new_state.set_string(&AttributePath::new("scsihw"), scsihw);
                }
                if let Some(ostype) = vm_config.ostype {
                    let _ = new_state.set_string(&AttributePath::new("ostype"), ostype);
                }
                if let Some(agent) = vm_config.agent {
                    let _ = new_state.set_string(&AttributePath::new("agent"), agent);
                }
                if let Some(onboot) = vm_config.onboot {
                    let _ = new_state.set_bool(&AttributePath::new("onboot"), onboot);
                }
                if let Some(tablet) = vm_config.tablet {
                    let _ = new_state.set_bool(&AttributePath::new("tablet"), tablet);
                }
                if let Some(protection) = vm_config.protection {
                    let _ = new_state.set_bool(&AttributePath::new("protection"), protection);
                }
                if let Some(tags) = vm_config.tags {
                    let _ = new_state.set_string(&AttributePath::new("tags"), tags);
                }
                if let Some(description) = vm_config.description {
                    let _ = new_state.set_string(&AttributePath::new("description"), description);
                }

                if let Some(scsi0) = vm_config.scsi0 {
                    let _ = new_state.set_string(&AttributePath::new("scsi0"), scsi0);
                }
                if let Some(scsi1) = vm_config.scsi1 {
                    let _ = new_state.set_string(&AttributePath::new("scsi1"), scsi1);
                }
                if let Some(scsi2) = vm_config.scsi2 {
                    let _ = new_state.set_string(&AttributePath::new("scsi2"), scsi2);
                }
                if let Some(scsi3) = vm_config.scsi3 {
                    let _ = new_state.set_string(&AttributePath::new("scsi3"), scsi3);
                }
                if let Some(virtio0) = vm_config.virtio0 {
                    let _ = new_state.set_string(&AttributePath::new("virtio0"), virtio0);
                }
                if let Some(virtio1) = vm_config.virtio1 {
                    let _ = new_state.set_string(&AttributePath::new("virtio1"), virtio1);
                }
                if let Some(ide0) = vm_config.ide0 {
                    let _ = new_state.set_string(&AttributePath::new("ide0"), ide0);
                }
                if let Some(ide2) = vm_config.ide2 {
                    let _ = new_state.set_string(&AttributePath::new("ide2"), ide2);
                }
                if let Some(sata0) = vm_config.sata0 {
                    let _ = new_state.set_string(&AttributePath::new("sata0"), sata0);
                }

                if let Some(net0) = vm_config.net0 {
                    let _ = new_state.set_string(&AttributePath::new("net0"), net0);
                }
                if let Some(net1) = vm_config.net1 {
                    let _ = new_state.set_string(&AttributePath::new("net1"), net1);
                }
                if let Some(net2) = vm_config.net2 {
                    let _ = new_state.set_string(&AttributePath::new("net2"), net2);
                }
                if let Some(net3) = vm_config.net3 {
                    let _ = new_state.set_string(&AttributePath::new("net3"), net3);
                }

                ReadResourceResponse {
                    new_state: Some(new_state),
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
                }
            }
            Err(crate::api::ApiError::ApiError { message, .. })
                if message.contains("does not exist") || message.contains("not found") =>
            {
                ReadResourceResponse {
                    new_state: None,
                    diagnostics,
                    private: request.private,
                    deferred: None,
                    new_identity: None,
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

        let node = match request.config.get_string(&AttributePath::new("node")) {
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

        let node = match request.prior_state.get_string(&AttributePath::new("node")) {
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

        match provider_data
            .client
            .nodes()
            .node(&node)
            .qemu()
            .delete(vmid, false)
            .await
        {
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
    fn extract_vm_config(
        &self,
        config: &DynamicValue,
    ) -> Result<(String, u32, crate::api::nodes::CreateQemuRequest), Diagnostic> {
        let node = config
            .get_string(&AttributePath::new("node"))
            .map_err(|_| Diagnostic::error("Missing node", "The 'node' attribute is required"))?;

        let vmid = config
            .get_number(&AttributePath::new("vmid"))
            .map_err(|_| Diagnostic::error("Missing vmid", "The 'vmid' attribute is required"))?
            as u32;

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

        let scsi0 = config.get_string(&AttributePath::new("scsi0")).ok();
        let scsi1 = config.get_string(&AttributePath::new("scsi1")).ok();
        let scsi2 = config.get_string(&AttributePath::new("scsi2")).ok();
        let scsi3 = config.get_string(&AttributePath::new("scsi3")).ok();
        let virtio0 = config.get_string(&AttributePath::new("virtio0")).ok();
        let virtio1 = config.get_string(&AttributePath::new("virtio1")).ok();
        let ide0 = config.get_string(&AttributePath::new("ide0")).ok();
        let ide2 = config.get_string(&AttributePath::new("ide2")).ok();
        let sata0 = config.get_string(&AttributePath::new("sata0")).ok();

        let net0 = config.get_string(&AttributePath::new("net0")).ok();
        let net1 = config.get_string(&AttributePath::new("net1")).ok();
        let net2 = config.get_string(&AttributePath::new("net2")).ok();
        let net3 = config.get_string(&AttributePath::new("net3")).ok();

        let create_request = crate::api::nodes::CreateQemuRequest {
            vmid,
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
            efidisk0: None,
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

        let scsi0 = config.get_string(&AttributePath::new("scsi0")).ok();
        let scsi1 = config.get_string(&AttributePath::new("scsi1")).ok();
        let scsi2 = config.get_string(&AttributePath::new("scsi2")).ok();
        let scsi3 = config.get_string(&AttributePath::new("scsi3")).ok();
        let virtio0 = config.get_string(&AttributePath::new("virtio0")).ok();
        let virtio1 = config.get_string(&AttributePath::new("virtio1")).ok();
        let ide0 = config.get_string(&AttributePath::new("ide0")).ok();
        let ide2 = config.get_string(&AttributePath::new("ide2")).ok();
        let sata0 = config.get_string(&AttributePath::new("sata0")).ok();

        let net0 = config.get_string(&AttributePath::new("net0")).ok();
        let net1 = config.get_string(&AttributePath::new("net1")).ok();
        let net2 = config.get_string(&AttributePath::new("net2")).ok();
        let net3 = config.get_string(&AttributePath::new("net3")).ok();

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
            efidisk0: None,
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
        let _ = state.set_string(&AttributePath::new("node"), node.to_string());
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
