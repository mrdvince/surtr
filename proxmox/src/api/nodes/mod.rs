//! Nodes API module for accessing node-specific resources

use crate::api::{client::Client, error::ApiError};
use serde::{Deserialize, Serialize};

mod qemu;
pub use qemu::{CreateQemuRequest, QemuApi, QemuConfig, QemuStatus, QemuVmInfo, UpdateQemuRequest};

pub struct NodesApi<'a> {
    client: &'a Client,
}

impl<'a> NodesApi<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<Vec<NodeStatus>, ApiError> {
        self.client.get("/api2/json/nodes").await
    }

    pub fn node(&self, node: &str) -> NodeApi<'a> {
        NodeApi {
            client: self.client,
            node: node.to_string(),
        }
    }
}

pub struct NodeApi<'a> {
    client: &'a Client,
    node: String,
}

impl<'a> NodeApi<'a> {
    pub fn qemu(&self) -> QemuApi<'a> {
        QemuApi::new(self.client, &self.node)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node: String,
    pub status: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub cpu: Option<f64>,
    pub maxcpu: Option<u32>,
    pub mem: Option<u64>,
    pub maxmem: Option<u64>,
    pub disk: Option<u64>,
    pub maxdisk: Option<u64>,
    pub uptime: Option<u64>,
}
