//! Provider data structure passed to resources and data sources

use crate::api::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxmoxProviderData {
    pub client: Arc<Client>,
}

impl ProxmoxProviderData {
    pub fn new(client: Client) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}
