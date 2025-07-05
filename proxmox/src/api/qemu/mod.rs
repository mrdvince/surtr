//! QEMU/KVM virtual machine API implementation

use crate::api::Client;
use serde::{Deserialize, Serialize};

/// QEMU API providing virtual machine operations
pub struct QemuApi<'a> {
    client: &'a Client,
}

impl<'a> QemuApi<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// GET /api2/json/nodes/{node}/qemu
    pub async fn list_vms(&self, node: &str) -> Result<Vec<VmListItem>, crate::api::ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu", node);
        self.client.get(&path).await
    }

    /// GET /api2/json/nodes/{node}/qemu/{vmid}/config
    pub async fn get_vm(&self, node: &str, vmid: u32) -> Result<VmConfig, crate::api::ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/config", node, vmid);
        self.client.get(&path).await
    }

    /// POST /api2/json/nodes/{node}/qemu
    pub async fn create_vm(
        &self,
        node: &str,
        config: &CreateVmRequest,
    ) -> Result<String, crate::api::ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu", node);
        self.client.post(&path, config).await
    }

    /// PUT /api2/json/nodes/{node}/qemu/{vmid}/config
    pub async fn update_vm(
        &self,
        node: &str,
        vmid: u32,
        config: &UpdateVmRequest,
    ) -> Result<(), crate::api::ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/config", node, vmid);
        self.client.put_no_response(&path, config).await
    }

    /// DELETE /api2/json/nodes/{node}/qemu/{vmid}
    pub async fn delete_vm(&self, node: &str, vmid: u32) -> Result<String, crate::api::ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}", node, vmid);
        self.client.delete(&path).await.map(|()| String::new())
    }

    /// GET /api2/json/nodes/{node}/qemu/{vmid}/status/current
    pub async fn get_vm_status(
        &self,
        node: &str,
        vmid: u32,
    ) -> Result<VmStatus, crate::api::ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/status/current", node, vmid);
        self.client.get(&path).await
    }
}

/// Item in VM list response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VmListItem {
    pub vmid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxdisk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxmem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netin: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diskread: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diskwrite: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qmpstatus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<u64>,
}

/// VM configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acpi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balloon: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootdisk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpulimit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpuunits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efidisk0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hookscript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotplug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localtime: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_downtime: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_speed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nameserver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onboot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ostype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reboot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi7: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsihw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub searchdomain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smbios1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tablet: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vga: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio7: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio8: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio9: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio10: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio11: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio12: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio13: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio14: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio15: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmgenid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmstatestorage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<String>,
}

/// Request for creating a VM
#[derive(Debug, Clone, Serialize)]
pub struct CreateVmRequest {
    pub vmid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acpi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balloon: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootdisk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpulimit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpuunits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efidisk0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hookscript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotplug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localtime: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_downtime: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_speed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nameserver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onboot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ostype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reboot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi7: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsihw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub searchdomain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smbios1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tablet: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vga: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio7: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio8: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio9: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio10: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio11: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio12: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio13: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio14: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio15: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmgenid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmstatestorage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<String>,
}

/// Request for updating a VM
#[derive(Debug, Clone, Serialize)]
pub struct UpdateVmRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acpi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balloon: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootdisk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpulimit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpuunits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efidisk0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hookscript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotplug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ide3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localtime: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_downtime: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrate_speed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nameserver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numa1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onboot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ostype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reboot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sata5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi7: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsihw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub searchdomain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smbios1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tablet: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vga: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio6: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio7: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio8: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio9: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio10: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio11: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio12: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio13: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio14: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio15: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmgenid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmstatestorage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<String>,
}

/// VM status information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VmStatus {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ha: Option<HaStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qmpstatus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxmem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxdisk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diskread: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diskwrite: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netin: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ballooninfo: Option<BalloonInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_qemu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_machine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockstat: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nics: Option<serde_json::Value>,
}

/// HA status information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HaStatus {
    pub managed: bool,
}

/// Balloon memory information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BalloonInfo {
    pub actual: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_mem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_mem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_mem: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_swapped_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_swapped_out: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major_page_faults: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor_page_faults: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update: Option<u64>,
}
