#[cfg(test)]
#[allow(unused_mut)]
mod tests {
    use super::super::*;
    use crate::api::test_helpers::create_test_client;
    use mockito::{Matcher, Server};

    #[test]
    fn test_qemu_api_new() {
        let client = create_test_client("https://test.example.com:8006");
        let api = QemuApi::new(&client, "node1");
        assert_eq!(api.node, "node1");
    }

    #[tokio::test]
    async fn test_list_vms_empty() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/api2/json/nodes/node1/qemu")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": []
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.list().await;

        assert!(result.is_ok());
        let vms = result.unwrap();
        assert!(vms.is_empty());
    }

    #[tokio::test]
    async fn test_list_vms_with_data() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/api2/json/nodes/node1/qemu")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": [
                    {
                        "vmid": 100,
                        "name": "test-vm",
                        "status": "running",
                        "cpu": 0.5,
                        "cpus": 2,
                        "maxmem": 2147483648,
                        "mem": 1073741824
                    },
                    {
                        "vmid": 101,
                        "status": "stopped"
                    }
                ]
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.list().await;

        assert!(result.is_ok());
        let vms = result.unwrap();
        assert_eq!(vms.len(), 2);

        assert_eq!(vms[0].vmid, 100);
        assert_eq!(vms[0].name, Some("test-vm".to_string()));
        assert_eq!(vms[0].status, "running");
        assert_eq!(vms[0].cpu, Some(0.5));
        assert_eq!(vms[0].cpus, Some(2));

        assert_eq!(vms[1].vmid, 101);
        assert_eq!(vms[1].name, None);
        assert_eq!(vms[1].status, "stopped");
    }

    #[tokio::test]
    async fn test_get_config() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/api2/json/nodes/node1/qemu/100/config")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": {
                    "name": "test-vm",
                    "cores": 2,
                    "memory": 2048,
                    "sockets": 1,
                    "cpu": "host",
                    "ostype": "l26"
                }
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.get_config(100).await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, Some("test-vm".to_string()));
        assert_eq!(config.cores, Some(2));
        assert_eq!(config.memory, Some(2048));
        assert_eq!(config.sockets, Some(1));
        assert_eq!(config.cpu, Some("host".to_string()));
        assert_eq!(config.ostype, Some("l26".to_string()));
    }

    #[tokio::test]
    async fn test_create_vm() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/api2/json/nodes/node1/qemu")
            .match_header("content-type", "application/json")
            .match_body(Matcher::JsonString(
                r#"{"vmid":100,"name":"test-vm","memory":2048}"#.to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": "UPID:node1:00001234:00000000:5F000000:qmcreate:100:root@pam:"
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");

        let request = CreateQemuRequest {
            vmid: 100,
            name: Some("test-vm".to_string()),
            memory: Some(2048),
            ..Default::default()
        };

        let result = api.create(100, &request).await;
        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert!(task_id.0.starts_with("UPID:"));
    }

    #[tokio::test]
    async fn test_update_config() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/api2/json/nodes/node1/qemu/100/config")
            .match_header("content-type", "application/json")
            .match_body(Matcher::JsonString(
                r#"{"memory":4096,"cores":4}"#.to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": null
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");

        let request = UpdateQemuRequest {
            memory: Some(4096),
            cores: Some(4),
            ..Default::default()
        };

        let result = api.update_config(100, &request).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_vm() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("DELETE", "/api2/json/nodes/node1/qemu/100?purge=1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": "UPID:node1:00001234:00000000:5F000000:qmdestroy:100:root@pam:"
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.delete(100, true).await;

        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert!(task_id.0.starts_with("UPID:"));
    }

    #[tokio::test]
    async fn test_delete_vm_no_purge() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("DELETE", "/api2/json/nodes/node1/qemu/100")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": "UPID:node1:00001234:00000000:5F000000:qmdestroy:100:root@pam:"
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.delete(100, false).await;

        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert!(task_id.0.starts_with("UPID:"));
    }

    #[tokio::test]
    async fn test_start_vm() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/api2/json/nodes/node1/qemu/100/status/start")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": "UPID:node1:00001234:00000000:5F000000:qmstart:100:root@pam:"
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.start(100).await;

        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert!(task_id.0.starts_with("UPID:"));
    }

    #[tokio::test]
    async fn test_stop_vm() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/api2/json/nodes/node1/qemu/100/status/stop")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": "UPID:node1:00001234:00000000:5F000000:qmstop:100:root@pam:"
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.stop(100).await;

        assert!(result.is_ok());
        let task_id = result.unwrap();
        assert!(task_id.0.starts_with("UPID:"));
    }

    #[tokio::test]
    async fn test_get_status() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/api2/json/nodes/node1/qemu/100/status/current")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": {
                    "status": "running",
                    "pid": 1234,
                    "uptime": 3600,
                    "cpu": 0.5,
                    "cpus": 2,
                    "mem": 1073741824,
                    "maxmem": 2147483648
                }
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.get_status(100).await;

        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.status, "running");
        assert_eq!(status.pid, Some(1234));
        assert_eq!(status.uptime, Some(3600));
        assert_eq!(status.cpu, Some(0.5));
        assert_eq!(status.cpus, Some(2));
        assert_eq!(status.mem, Some(1073741824));
        assert_eq!(status.maxmem, Some(2147483648));
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/api2/json/nodes/node1/qemu/100/config")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "data": null,
                "errors": {
                    "vmid": "VM 100 not found"
                }
            }"#,
            )
            .create_async()
            .await;

        let client = create_test_client(&server.url());
        let api = QemuApi::new(&client, "node1");
        let result = api.get_config(100).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("404"));
        }
    }
}
