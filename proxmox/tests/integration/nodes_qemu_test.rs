//! Integration tests for Proxmox QEMU/VM operations

use proxmox::api::nodes::{CreateQemuRequest, UpdateQemuRequest};
use proxmox::api::Client;
use std::sync::Arc;

fn init_test_logger() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init();
}

async fn wait_for_vm_ready(
    client: &Client,
    node: &str,
    vmid: u32,
    max_attempts: u32,
) -> Result<(), String> {
    println!("Waiting for VM {} to be ready...", vmid);

    for attempt in 1..=max_attempts {
        println!("Attempt {}/{} to read VM config", attempt, max_attempts);
        match client.nodes().node(node).qemu().get_config(vmid).await {
            Ok(config) => {
                println!(
                    "Successfully read VM config: name={:?}, memory={:?}, cores={:?}",
                    config.name, config.memory, config.cores
                );
                // Check if the VM config is properly populated
                if config.name.is_some() || config.memory.is_some() {
                    println!("VM config is populated, VM is ready");
                    return Ok(());
                } else {
                    println!("VM config exists but is not fully populated yet");
                }
            }
            Err(e) => {
                println!("Failed to read VM config (attempt {}): {:?}", attempt, e);
            }
        }

        if attempt < max_attempts {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    Err(format!(
        "VM {} not ready after {} attempts",
        vmid, max_attempts
    ))
}

#[tokio::test]
#[ignore] // Only run with actual Proxmox instance
async fn test_qemu_vm_lifecycle() {
    init_test_logger();

    let endpoint = std::env::var("PROXMOX_ENDPOINT").expect("PROXMOX_ENDPOINT not set");
    let api_token = std::env::var("PROXMOX_API_TOKEN").expect("PROXMOX_API_TOKEN not set");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap();

    println!("Connecting to Proxmox at: {}", endpoint);
    let client = Client::new(&endpoint, &api_token, insecure).expect("Failed to create client");
    let client = Arc::new(client);

    let node = std::env::var("PROXMOX_TEST_NODE").unwrap_or_else(|_| "mjolnir".to_string());
    let test_vmid = 9999;

    // First, try to delete any existing VM with this ID
    println!("Cleaning up any existing VM with ID {}", test_vmid);
    match client
        .nodes()
        .node(&node)
        .qemu()
        .delete(test_vmid, true)
        .await
    {
        Ok(_) => {
            println!("Deleted existing VM, waiting for cleanup...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        Err(e) => println!("No existing VM to delete or delete failed: {:?}", e),
    }

    // Step 1: Create VM
    println!("Creating VM with ID {}", test_vmid);
    let create_request = CreateQemuRequest {
        vmid: test_vmid,
        name: Some("test-vm-integration".to_string()),
        memory: Some(1024),
        cores: Some(1),
        sockets: Some(1),
        cpu: Some("host".to_string()),
        ostype: Some("l26".to_string()),
        ..Default::default()
    };

    let create_result = client
        .nodes()
        .node(&node)
        .qemu()
        .create(test_vmid, &create_request)
        .await;

    match &create_result {
        Ok(task_id) => println!("VM creation started, task ID: {:?}", task_id),
        Err(e) => println!("Failed to create VM: {:?}", e),
    }

    assert!(
        create_result.is_ok(),
        "Failed to create VM: {:?}",
        create_result
    );

    // Wait for VM to be ready
    wait_for_vm_ready(&client, &node, test_vmid, 10)
        .await
        .expect("VM did not become ready in time");

    // Step 2: Read VM configuration
    println!("Reading VM configuration");
    let read_result = client
        .nodes()
        .node(&node)
        .qemu()
        .get_config(test_vmid)
        .await;

    assert!(
        read_result.is_ok(),
        "Failed to read VM config: {:?}",
        read_result
    );
    let vm_config = read_result.unwrap();
    println!(
        "VM config retrieved: name={:?}, memory={:?}",
        vm_config.name, vm_config.memory
    );
    assert_eq!(vm_config.name, Some("test-vm-integration".to_string()));
    assert_eq!(vm_config.memory, Some(1024));

    // Step 3: Update VM
    let update_request = UpdateQemuRequest {
        memory: Some(2048),
        cores: Some(2),
        ..Default::default()
    };

    let update_result = client
        .nodes()
        .node(&node)
        .qemu()
        .update_config(test_vmid, &update_request)
        .await;

    assert!(
        update_result.is_ok(),
        "Failed to update VM: {:?}",
        update_result
    );

    // Wait for update to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Step 4: Verify update
    let read_result = client
        .nodes()
        .node(&node)
        .qemu()
        .get_config(test_vmid)
        .await;

    assert!(read_result.is_ok());
    let updated_config = read_result.unwrap();
    assert_eq!(updated_config.memory, Some(2048));
    assert_eq!(updated_config.cores, Some(2));

    // Step 5: Delete VM
    let delete_result = client
        .nodes()
        .node(&node)
        .qemu()
        .delete(test_vmid, true)
        .await;

    assert!(
        delete_result.is_ok(),
        "Failed to delete VM: {:?}",
        delete_result
    );
    let delete_task_id = delete_result.unwrap();
    println!("Delete task ID: {:?}", delete_task_id);
}

#[tokio::test]
#[ignore] // Only run with actual Proxmox instance
async fn test_qemu_list_vms() {
    let endpoint = std::env::var("PROXMOX_ENDPOINT").expect("PROXMOX_ENDPOINT not set");
    let api_token = std::env::var("PROXMOX_API_TOKEN").expect("PROXMOX_API_TOKEN not set");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap();

    let client = Client::new(&endpoint, &api_token, insecure).expect("Failed to create client");
    let node = std::env::var("PROXMOX_TEST_NODE").unwrap_or_else(|_| "mjolnir".to_string());

    let list_result = client.nodes().node(&node).qemu().list().await;

    assert!(list_result.is_ok(), "Failed to list VMs: {:?}", list_result);
    let vms = list_result.unwrap();
    println!("Found {} VMs on node {}", vms.len(), node);

    for vm in vms {
        println!(
            "VM: {} (ID: {}), Status: {}",
            vm.name.unwrap_or_else(|| "unnamed".to_string()),
            vm.vmid,
            vm.status
        );
    }
}

#[tokio::test]
#[ignore] // Only run with actual Proxmox instance
async fn test_nodes_list() {
    let endpoint = std::env::var("PROXMOX_ENDPOINT").expect("PROXMOX_ENDPOINT not set");
    let api_token = std::env::var("PROXMOX_API_TOKEN").expect("PROXMOX_API_TOKEN not set");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap();

    let client = Client::new(&endpoint, &api_token, insecure).expect("Failed to create client");

    let list_result = client.nodes().list().await;

    assert!(
        list_result.is_ok(),
        "Failed to list nodes: {:?}",
        list_result
    );
    let nodes = list_result.unwrap();
    assert!(!nodes.is_empty(), "No nodes found");

    for node in nodes {
        println!(
            "Node: {}, Status: {}, Type: {}",
            node.node, node.status, node.type_
        );
    }
}

#[tokio::test]
#[ignore] // Only run with actual Proxmox instance
async fn test_qemu_vm_status() {
    init_test_logger();

    let endpoint = std::env::var("PROXMOX_ENDPOINT").expect("PROXMOX_ENDPOINT not set");
    let api_token = std::env::var("PROXMOX_API_TOKEN").expect("PROXMOX_API_TOKEN not set");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap();

    println!("Connecting to Proxmox at: {}", endpoint);
    let client = Client::new(&endpoint, &api_token, insecure).expect("Failed to create client");
    let node = std::env::var("PROXMOX_TEST_NODE").unwrap_or_else(|_| "mjolnir".to_string());
    let test_vmid = 9998;

    // First, try to delete any existing VM with this ID
    println!("Cleaning up any existing VM with ID {}", test_vmid);
    match client
        .nodes()
        .node(&node)
        .qemu()
        .delete(test_vmid, true)
        .await
    {
        Ok(_) => {
            println!("Deleted existing VM, waiting for cleanup...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        Err(e) => println!("No existing VM to delete or delete failed: {:?}", e),
    }

    // Create a test VM first
    println!("Creating test VM with ID {}", test_vmid);
    let create_request = CreateQemuRequest {
        vmid: test_vmid,
        name: Some("test-vm-status".to_string()),
        memory: Some(512),
        cores: Some(1),
        ..Default::default()
    };

    let create_result = client
        .nodes()
        .node(&node)
        .qemu()
        .create(test_vmid, &create_request)
        .await;

    match &create_result {
        Ok(task_id) => println!("VM creation started, task ID: {:?}", task_id),
        Err(e) => panic!("Failed to create test VM: {:?}", e),
    }

    // Wait for VM to be ready
    wait_for_vm_ready(&client, &node, test_vmid, 10)
        .await
        .expect("VM did not become ready in time");

    // Get VM status
    println!("Getting VM status");
    let status_result = client
        .nodes()
        .node(&node)
        .qemu()
        .get_status(test_vmid)
        .await;

    match &status_result {
        Ok(status) => println!("VM Status response: {:?}", status),
        Err(e) => println!("Failed to get VM status: {:?}", e),
    }

    assert!(
        status_result.is_ok(),
        "Failed to get VM status: {:?}",
        status_result
    );
    let status = status_result.unwrap();
    println!("VM Status: {}", status.status);

    // Start VM
    println!("Starting VM");
    let start_result = client.nodes().node(&node).qemu().start(test_vmid).await;

    match &start_result {
        Ok(task_id) => println!("VM start task ID: {:?}", task_id),
        Err(e) => println!("Failed to start VM: {:?}", e),
    }

    assert!(
        start_result.is_ok(),
        "Failed to start VM: {:?}",
        start_result
    );

    // Wait for VM to start
    println!("Waiting for VM to start...");
    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    // Check status again
    println!("Checking VM status after start");
    let status_result = client
        .nodes()
        .node(&node)
        .qemu()
        .get_status(test_vmid)
        .await;

    assert!(
        status_result.is_ok(),
        "Failed to get VM status after start: {:?}",
        status_result
    );
    let status = status_result.unwrap();
    println!("VM Status after start: {}", status.status);
    assert_eq!(status.status, "running");

    // Stop VM
    println!("Stopping VM");
    let stop_result = client.nodes().node(&node).qemu().stop(test_vmid).await;

    assert!(stop_result.is_ok(), "Failed to stop VM: {:?}", stop_result);

    // Wait for VM to stop
    println!("Waiting for VM to stop...");
    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    // Clean up
    println!("Cleaning up VM");
    match client
        .nodes()
        .node(&node)
        .qemu()
        .delete(test_vmid, true)
        .await
    {
        Ok(_) => println!("VM deleted successfully"),
        Err(e) => println!("Failed to delete VM: {:?}", e),
    }
}
