use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub release: String,
    pub repoid: String,
}

impl super::Client {
    pub async fn get_version(&self) -> Result<VersionInfo, super::ApiError> {
        self.get("/api2/json/version").await
    }
}
