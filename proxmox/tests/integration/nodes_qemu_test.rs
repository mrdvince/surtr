//! Integration tests for Proxmox QEMU/VM operations

use proxmox::api::Client;
use proxmox::api::nodes::{CreateQemuRequest, UpdateQemuRequest};
use std::sync::Arc;

#[tokio::test]
#[ignore] // Only run with actual Proxmox instance
async fn test_qemu_vm_lifecycle() {
    let endpoint = std::env::var("PROXMOX_ENDPOINT").expect("PROXMOX_ENDPOINT not set");
    let api_token = std::env::var("PROXMOX_API_TOKEN").expect("PROXMOX_API_TOKEN not set");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap();
    
    let client = Client::new(&endpoint, &api_token, insecure).expect("Failed to create client");
    let client = Arc::new(client);
    
    let node = "pve"; // Adjust based on your test environment
    let test_vmid = 9999;
    
    // Step 1: Create VM
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
        .node(node)
        .qemu()
        .create(test_vmid, &create_request)
        .await;
    
    assert!(create_result.is_ok(), "Failed to create VM: {:?}", create_result);
    let task_id = create_result.unwrap();
    println!("Create task ID: {:?}", task_id);
    
    // Wait for creation to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Step 2: Read VM configuration
    let read_result = client
        .nodes()
        .node(node)
        .qemu()
        .get_config(test_vmid)
        .await;
    
    assert!(read_result.is_ok(), "Failed to read VM config: {:?}", read_result);
    let vm_config = read_result.unwrap();
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
        .node(node)
        .qemu()
        .update_config(test_vmid, &update_request)
        .await;
    
    assert!(update_result.is_ok(), "Failed to update VM: {:?}", update_result);
    
    // Wait for update to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Step 4: Verify update
    let read_result = client
        .nodes()
        .node(node)
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
        .node(node)
        .qemu()
        .delete(test_vmid, true)
        .await;
    
    assert!(delete_result.is_ok(), "Failed to delete VM: {:?}", delete_result);
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
    let node = "pve"; // Adjust based on your test environment
    
    let list_result = client
        .nodes()
        .node(node)
        .qemu()
        .list()
        .await;
    
    assert!(list_result.is_ok(), "Failed to list VMs: {:?}", list_result);
    let vms = list_result.unwrap();
    println!("Found {} VMs on node {}", vms.len(), node);
    
    for vm in vms {
        println!("VM: {} (ID: {}), Status: {}", 
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
    
    assert!(list_result.is_ok(), "Failed to list nodes: {:?}", list_result);
    let nodes = list_result.unwrap();
    assert!(!nodes.is_empty(), "No nodes found");
    
    for node in nodes {
        println!("Node: {}, Status: {}, Type: {}", 
            node.node,
            node.status,
            node.type_
        );
    }
}

#[tokio::test]
#[ignore] // Only run with actual Proxmox instance
async fn test_qemu_vm_status() {
    let endpoint = std::env::var("PROXMOX_ENDPOINT").expect("PROXMOX_ENDPOINT not set");
    let api_token = std::env::var("PROXMOX_API_TOKEN").expect("PROXMOX_API_TOKEN not set");
    let insecure = std::env::var("PROXMOX_INSECURE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap();
    
    let client = Client::new(&endpoint, &api_token, insecure).expect("Failed to create client");
    let node = "pve";
    let test_vmid = 9998;
    
    // Create a test VM first
    let create_request = CreateQemuRequest {
        vmid: test_vmid,
        name: Some("test-vm-status".to_string()),
        memory: Some(512),
        cores: Some(1),
        ..Default::default()
    };
    
    let _ = client
        .nodes()
        .node(node)
        .qemu()
        .create(test_vmid, &create_request)
        .await
        .expect("Failed to create test VM");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Get VM status
    let status_result = client
        .nodes()
        .node(node)
        .qemu()
        .get_status(test_vmid)
        .await;
    
    assert!(status_result.is_ok(), "Failed to get VM status: {:?}", status_result);
    let status = status_result.unwrap();
    println!("VM Status: {}", status.status);
    
    // Start VM
    let start_result = client
        .nodes()
        .node(node)
        .qemu()
        .start(test_vmid)
        .await;
    
    assert!(start_result.is_ok(), "Failed to start VM: {:?}", start_result);
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Check status again
    let status_result = client
        .nodes()
        .node(node)
        .qemu()
        .get_status(test_vmid)
        .await;
    
    assert!(status_result.is_ok());
    let status = status_result.unwrap();
    assert_eq!(status.status, "running");
    
    // Stop VM
    let stop_result = client
        .nodes()
        .node(node)
        .qemu()
        .stop(test_vmid)
        .await;
    
    assert!(stop_result.is_ok(), "Failed to stop VM: {:?}", stop_result);
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Clean up
    let _ = client
        .nodes()
        .node(node)
        .qemu()
        .delete(test_vmid, true)
        .await;
}