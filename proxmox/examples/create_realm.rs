use reqwest::ClientBuilder;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();

    // Get environment variables
    let endpoint = std::env::var("PROXMOX_ENDPOINT")
        .expect("PROXMOX_ENDPOINT environment variable is required");
    let api_token = std::env::var("PROXMOX_API_TOKEN")
        .expect("PROXMOX_API_TOKEN environment variable is required");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    info!("Endpoint: {}", endpoint);
    info!("API Token: {}", api_token);
    info!("API Token bytes: {:?}", api_token.as_bytes());
    info!("Insecure: {}", insecure);

    // Create HTTP client with same config as our API client
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(insecure)
        .timeout(Duration::from_secs(30))
        .build()?;

    // Prepare form data - same as in our API client
    let mut form_data = HashMap::new();
    form_data.insert("realm", "test-oidc");
    form_data.insert("type", "openid");
    form_data.insert("issuer-url", "https://auth.example.com");
    form_data.insert("client-id", "proxmox-test");
    form_data.insert("client-key", "super-secret-key");
    form_data.insert("username-claim", "email");
    form_data.insert("autocreate", "1"); // Boolean as "1"
    form_data.insert("default", "0"); // Boolean as "0"
    form_data.insert("comment", "Test OIDC realm created by standalone example");

    let url = format!(
        "{}/api2/json/access/domains",
        endpoint.trim_end_matches('/')
    );
    let auth_header = format!("PVEAPIToken={}", api_token);

    info!("URL: {}", url);
    info!("Authorization header: {}", auth_header);
    info!("Form data: {:?}", form_data);

    // Make the request
    let response = client
        .post(&url)
        .header("Authorization", auth_header)
        .form(&form_data)
        .send()
        .await?;

    let status = response.status();
    info!("Response status: {}", status);

    // Get response headers
    info!("Response headers:");
    for (name, value) in response.headers() {
        info!("  {}: {:?}", name, value);
    }

    // Get response body
    let body = response.text().await?;
    info!("Response body: {}", body);

    if status.is_success() {
        info!("✓ Realm created successfully!");
    } else if status == 401 {
        error!("✗ Authentication failed (401)");
        error!("Please check your API token format: user@realm!tokenid=secret");
    } else if status == 400 && body.contains("already exists") {
        info!("Realm already exists, trying to delete it first...");

        // Try to delete the existing realm
        let delete_url = format!(
            "{}/api2/json/access/domains/test-oidc",
            endpoint.trim_end_matches('/')
        );
        let delete_response = client
            .delete(&delete_url)
            .header("Authorization", format!("PVEAPIToken={}", api_token))
            .send()
            .await?;

        let delete_status = delete_response.status();
        let delete_body = delete_response.text().await?;

        if delete_status.is_success() {
            info!("✓ Deleted existing realm");
            info!("Now retry creating the realm...");

            // Retry creation
            let retry_response = client
                .post(&url)
                .header("Authorization", format!("PVEAPIToken={}", api_token))
                .form(&form_data)
                .send()
                .await?;

            let retry_status = retry_response.status();
            let retry_body = retry_response.text().await?;

            info!("Retry status: {}", retry_status);
            info!("Retry body: {}", retry_body);

            if retry_status.is_success() {
                info!("✓ Realm created successfully on retry!");
            } else {
                error!("✗ Failed to create realm on retry");
            }
        } else {
            error!(
                "✗ Failed to delete existing realm: {} - {}",
                delete_status, delete_body
            );
        }
    } else {
        error!("✗ Failed with status: {}", status);
    }

    // Also test a simple GET request to verify auth works
    info!("\nTesting GET request to /version endpoint...");
    let version_url = format!("{}/api2/json/version", endpoint.trim_end_matches('/'));
    let version_response = client
        .get(&version_url)
        .header("Authorization", format!("PVEAPIToken={}", api_token))
        .send()
        .await?;

    let version_status = version_response.status();
    let version_body = version_response.text().await?;

    info!("Version endpoint status: {}", version_status);
    info!("Version endpoint body: {}", version_body);

    Ok(())
}
