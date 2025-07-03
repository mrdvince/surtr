pub mod realms;

use crate::api::Client;

/// Access API providing authentication-related operations
pub struct AccessApi<'a> {
    client: &'a Client,
}

impl<'a> AccessApi<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Access realm operations
    pub fn realms(&self) -> realms::RealmsApi<'a> {
        realms::RealmsApi::new(self.client)
    }
}
