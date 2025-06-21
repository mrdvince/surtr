use proxmox::ProxmoxProvider;
use std::env;
use tfplug::grpc::ProviderServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let exe_dir = env::current_exe()?.parent().unwrap().to_path_buf();
    // TODO: Make TLS optional - only needed for local development
    let cert_path = exe_dir.join("../../certs/localhost+2.pem");
    let key_path = exe_dir.join("../../certs/localhost+2-key.pem");

    let provider = ProxmoxProvider::new();
    let server = ProviderServer::new(provider, cert_path, key_path);
    
    server.run().await?;
    
    Ok(())
}