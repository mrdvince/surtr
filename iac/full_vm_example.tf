resource "proxmox_qemu_vm" "this" {
  node = "mjolnir"
  vmid = 9001
  name = "athos-01"
  
  # Hardware configuration
  cores   = 2
  sockets = 1
  memory  = 2048
  cpu     = "x86-64-v2-AES"
  
  # BIOS and boot settings
  bios   = "ovmf"
  boot   = "order=scsi0;ide2;net0"
  
  # SCSI controller
  scsihw = "virtio-scsi-pci"
  
  # Operating system
  ostype = "l26"
  
  # Primary disk
  scsi0 = "local-lvm:20,format=raw"
  
  # ISO for installation
  ide2 = "local:iso/ubuntu-24.04.1-live-server-amd64.iso,media=cdrom"
  
  # Network configuration with MAC address
  net0 = "virtio=ba:88:cb:76:75:d6,bridge=vmbr0,firewall=0,tag=30"
  
  # Cloud-init configuration
  ciuser    = "ubuntu"
  sshkeys   = file("~/.ssh/devkey.pub")
  ipconfig0 = "ip=dhcp"
  
  # VM behavior
  agent      = "1"
  onboot     = true
  protection = false
  
  # Tags and metadata
  tags        = "controlplane,talos"
  description = "Talos control plane node - athos-01"
}

# Additional VMs with different configurations
resource "proxmox_qemu_vm" "web_server" {
  node = "mjolnir"
  vmid = 9002
  name = "web-server-01"
  
  # Hardware
  cores  = 1
  memory = 1024
  cpu    = "x86-64-v2-AES"
  
  # Storage
  scsi0 = "local-lvm:10,format=raw,iothread=1"
  
  # Network - Multiple interfaces
  net0 = "virtio,bridge=vmbr0,tag=50"      # Public network
  net1 = "virtio,bridge=vmbr1,tag=100"     # Private network
  
  # Cloud-init with static IP
  ciuser    = "webadmin"
  sshkeys   = file("~/.ssh/devkey.pub")
  ipconfig0 = "ip=192.168.50.100/24,gw=192.168.50.1"
  ipconfig1 = "ip=10.0.100.10/24"
  
  agent  = "1"
  onboot = true
  
  tags = "web,production"
}

# VM with full cloud-init configuration
resource "proxmox_qemu_vm" "database" {
  node = "mjolnir"
  vmid = 9003
  name = "postgres-01"
  
  # Hardware
  cores  = 1
  memory = 512
  cpu    = "x86-64-v2-AES"
  
  # Storage
  scsi0 = "local-lvm:10,format=raw"
  
  # Network
  net0 = "virtio,bridge=vmbr0,tag=100,firewall=1"
  
  # Full cloud-init
  ciuser      = "dbadmin"
  cipassword  = "changeme123!"  # Should use variables in production
  sshkeys     = file("~/.ssh/devkey.pub")
  ipconfig0   = "ip=192.168.100.50/24,gw=192.168.100.1"
  
  # VM settings
  agent  = "1"
  
  tags = "database,postgres"
}

# Outputs
output "vm_info" {
  value = {
    this = {
      name = proxmox_qemu_vm.this.name
      node = proxmox_qemu_vm.this.node
      vmid = proxmox_qemu_vm.this.vmid
    }
    web_server = {
      name = proxmox_qemu_vm.web_server.name
      node = proxmox_qemu_vm.web_server.node
      vmid = proxmox_qemu_vm.web_server.vmid
    }
    database = {
      name = proxmox_qemu_vm.database.name
      node = proxmox_qemu_vm.database.node
      vmid = proxmox_qemu_vm.database.vmid
    }
  }
}