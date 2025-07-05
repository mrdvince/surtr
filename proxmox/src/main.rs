use proxmox::ProxmoxProvider;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| "Failed to install rustls crypto provider")?;

    let mut config = tfplug::ServerConfig::default();

    let exe_dir = env::current_exe()?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();

    let cert_path = exe_dir.join("../../certs/localhost.pem");
    let key_path = exe_dir.join("../../certs/localhost-key.pem");

    config.cert_path = cert_path;
    config.key_path = key_path;

    let provider = ProxmoxProvider::new();
    tfplug::serve(provider, config).await?;

    Ok(())
}
