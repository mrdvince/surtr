resource "proxmox_qemu_vm" "this" {
  node = "mjolnir"
  vmid = 9001
  name = "athos-01"

  cores   = 2
  sockets = 1
  memory  = 2048
  cpu     = "x86-64-v2-AES"

  bios = "ovmf"
  boot = "order=scsi0;ide2;net0"
  efidisk0 = "local-lvm:1,format=qcow2"  # Required for OVMF BIOS

  scsihw      = "virtio-scsi-pci"
  ostype      = "l26"
  scsi0       = "local-lvm:20,format=raw"
  ide2        = "local:iso/ubuntu-24.04.1-live-server-amd64.iso,media=cdrom"
  net0        = "virtio=ba:88:cb:76:75:d6,bridge=vmbr0,firewall=0,tag=30"
  ciuser      = "ubuntu"
  sshkeys     = file("~/.ssh/devkey.pub")
  ipconfig0   = "ip=dhcp"
  agent       = "1"
  onboot      = true
  start       = true  # Start VM immediately after creation
  protection  = false
  tags        = "cp,"
  description = "A control plane node"
}

resource "proxmox_qemu_vm" "web_server" {
  node   = "mjolnir"
  vmid   = 9002
  name   = "web-server-01"
  cores  = 1
  memory = 1024
  cpu    = "x86-64-v2-AES"
  
  # Note: iothread requires virtio-scsi-single controller
  scsihw = "virtio-scsi-single"  # Added to support iothread
  scsi0  = "local-lvm:10,format=raw,iothread=1"

  # Network - Multiple interfaces
  net0 = "virtio,bridge=vmbr0,tag=50"
  net1 = "virtio,bridge=vmbr0,tag=100"  # Changed from vmbr1 to vmbr0

  ciuser    = "webadmin"
  sshkeys   = file("~/.ssh/devkey.pub")
  ipconfig0 = "ip=192.168.50.100/24,gw=192.168.50.1"
  ipconfig1 = "ip=10.0.100.10/24"

  agent  = "1"
  onboot = true
  start  = true

  tags = "web,production"
}

resource "proxmox_qemu_vm" "database" {
  node       = "mjolnir"
  vmid       = 9003
  name       = "postgres-01"
  cores      = 1
  memory     = 512
  cpu        = "x86-64-v2-AES"
  scsi0      = "local-lvm:10,format=raw"
  net0       = "virtio,bridge=vmbr0,tag=100,firewall=1"
  ciuser     = "dbadmin"
  cipassword = "changeme123!"
  sshkeys    = file("~/.ssh/devkey.pub")
  ipconfig0  = "ip=192.168.100.50/24,gw=192.168.100.1"
  agent      = "1"
  start      = true
  tags       = "database,postgres"
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