//! QEMU/KVM virtual machine API implementation

use crate::api::{common::TaskId, error::ApiError, Client};
use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_optional_string_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU64 {
        String(String),
        U64(u64),
    }

    match Option::<StringOrU64>::deserialize(deserializer)? {
        Some(StringOrU64::String(s)) => {
            s.parse::<u64>().map(Some).map_err(serde::de::Error::custom)
        }
        Some(StringOrU64::U64(u)) => Ok(Some(u)),
        None => Ok(None),
    }
}

fn deserialize_optional_string_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU32 {
        String(String),
        U32(u32),
    }

    match Option::<StringOrU32>::deserialize(deserializer)? {
        Some(StringOrU32::String(s)) => {
            s.parse::<u32>().map(Some).map_err(serde::de::Error::custom)
        }
        Some(StringOrU32::U32(u)) => Ok(Some(u)),
        None => Ok(None),
    }
}

/// QEMU API providing virtual machine operations
pub struct QemuApi<'a> {
    client: &'a Client,
    node: String,
}

impl<'a> QemuApi<'a> {
    pub fn new(client: &'a Client, node: &str) -> Self {
        Self {
            client,
            node: node.to_string(),
        }
    }

    /// GET /api2/json/nodes/{node}/qemu
    pub async fn list(&self) -> Result<Vec<QemuVmInfo>, ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu", self.node);
        self.client.get(&path).await
    }

    /// GET /api2/json/nodes/{node}/qemu/{vmid}/config
    pub async fn get_config(&self, vmid: u32) -> Result<QemuConfig, ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/config", self.node, vmid);
        self.client.get(&path).await
    }

    /// POST /api2/json/nodes/{node}/qemu
    pub async fn create(
        &self,
        _vmid: u32,
        request: &CreateQemuRequest,
    ) -> Result<TaskId, ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu", self.node);
        self.client.post(&path, request).await
    }

    /// POST /api2/json/nodes/{node}/qemu/{vmid}/config
    pub async fn update_config(
        &self,
        vmid: u32,
        request: &UpdateQemuRequest,
    ) -> Result<Option<TaskId>, ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/config", self.node, vmid);
        self.client.post(&path, request).await
    }

    /// DELETE /api2/json/nodes/{node}/qemu/{vmid}
    pub async fn delete(&self, vmid: u32, purge: bool) -> Result<TaskId, ApiError> {
        let path = if purge {
            format!("/api2/json/nodes/{}/qemu/{}?purge=1", self.node, vmid)
        } else {
            format!("/api2/json/nodes/{}/qemu/{}", self.node, vmid)
        };
        self.client.delete(&path).await
    }

    /// POST /api2/json/nodes/{node}/qemu/{vmid}/status/start
    pub async fn start(&self, vmid: u32) -> Result<TaskId, ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/status/start", self.node, vmid);
        self.client.post(&path, &()).await
    }

    /// POST /api2/json/nodes/{node}/qemu/{vmid}/status/stop
    pub async fn stop(&self, vmid: u32) -> Result<TaskId, ApiError> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/status/stop", self.node, vmid);
        self.client.post(&path, &()).await
    }

    /// GET /api2/json/nodes/{node}/qemu/{vmid}/status/current
    pub async fn get_status(&self, vmid: u32) -> Result<QemuStatus, ApiError> {
        let path = format!(
            "/api2/json/nodes/{}/qemu/{}/status/current",
            self.node, vmid
        );
        self.client.get(&path).await
    }
}

/// Item in VM list response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QemuVmInfo {
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
pub struct QemuConfig {
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_string_u32",
        default
    )]
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_string_u64",
        default
    )]
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
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateQemuRequest {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<bool>,
}

/// Request for updating a VM
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateQemuRequest {
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
pub struct QemuStatus {
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
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub managed: bool,
}

fn deserialize_bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolOrInt {
        Bool(bool),
        Int(u8),
    }

    match BoolOrInt::deserialize(deserializer)? {
        BoolOrInt::Bool(b) => Ok(b),
        BoolOrInt::Int(0) => Ok(false),
        BoolOrInt::Int(1) => Ok(true),
        BoolOrInt::Int(_) => Err(serde::de::Error::custom("expected 0 or 1")),
    }
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

#[cfg(test)]
#[path = "./qemu_test.rs"]
mod qemu_test;
