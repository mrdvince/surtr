resource "proxmox_qemu_vm" "ubuntu_vm" {
  node   = "mjolnir"
  vmid   = 101
  name   = "ubuntu-test"
  
  # Hardware configuration
  cores   = 2
  sockets = 1
  memory  = 2048
  cpu     = "x86-64-v2-AES"
  
  # BIOS and boot
  bios   = "ovmf"
  boot   = "order=scsi0;ide2;net0"
  scsihw = "virtio-scsi-single"
  
  # Operating system
  ostype = "l26"
  
  # Storage devices
  scsi0 = "local-lvm:32,format=raw,iothread=1"
  ide2  = "local:iso/ubuntu-24.04.1-live-server-amd64.iso,media=cdrom"
  
  # Network
  net0 = "virtio,bridge=vmbr0,firewall=1,tag=50"
  
  # Cloud-init configuration (optional)
  # ciuser  = "ubuntu"
  # sshkeys = file("~/.ssh/id_rsa.pub")
  # ipconfig0 = "ip=dhcp"
  
  # VM behavior
  agent   = "1"
  onboot  = true
  
  # Metadata
  tags        = "test,ubuntu"
  description = "Ubuntu test VM created with Terraform"
}

# Example with minimal configuration
resource "proxmox_qemu_vm" "minimal_vm" {
  node   = "mjolnir"
  vmid   = 102
  name   = "minimal-test"
  
  cores  = 1
  memory = 1024
  
  scsi0 = "local-lvm:10"
}

# Example with multiple disks and networks
resource "proxmox_qemu_vm" "multi_disk_vm" {
  node   = "mjolnir"
  vmid   = 103
  name   = "multi-disk-test"
  
  cores  = 4
  memory = 4096
  
  # Multiple storage devices
  scsi0 = "local-lvm:20,format=raw"
  scsi1 = "local-lvm:50,format=raw"
  
  # Multiple network interfaces
  net0 = "virtio,bridge=vmbr0"
  net1 = "virtio,bridge=vmbr1,tag=100"
  
  agent  = "1"
}

# Output VM information
output "ubuntu_vm_info" {
  value = {
    node = proxmox_qemu_vm.ubuntu_vm.node
    vmid = proxmox_qemu_vm.ubuntu_vm.vmid
    name = proxmox_qemu_vm.ubuntu_vm.name
  }
}