variable "instances" {
  description = "VM instances to create"
  type = list(object({
    vmname   = string
    vmid     = number
    ipconfig = string
    macaddr  = optional(string)
  }))
  default = [{
    vmname   = "test-01"
    vmid     = 9001
    ipconfig = "ip=dhcp"
    macaddr  = "ba:88:cb:76:75:d6"
  },
  {
    vmname   = "test-02"
    vmid     = 9002
    ipconfig = "ip=dhcp"
  }]
}

variable "template_name" {
  description = "Name of the template to clone from"
  type        = string
  default     = null
}

variable "os_type" {
  description = "Type of the OS (e.g., cloud_init)"
  type        = string
  default = null
}

variable "target_node" {
  description = "Proxmox node to deploy the VM to"
  type        = string
  default     = "mjolnir"
}

variable "tags" {
  description = "Tags for the VM"
  type        = string
  default     = "controlplane,talos"
}

variable "vm_config_map" {
  description = "VM hardware configuration"
  type = object({
    bios                   = string
    boot                   = string
    bootdisk               = string
    cores                  = number
    memory                 = number
    balloon                = number
    machine                = string
    scsihw                 = string
    onboot                 = bool
    define_connection_info = bool
    ciuser                 = optional(string)
    ciupgrade              = optional(bool)
  })
  default = {
    bios                   = "ovmf"
    boot                   = "c"
    bootdisk               = "ide2"
    cores                  = 2
    memory                 = 4096
    balloon                = 4096
    machine                = "q35"
    scsihw                 = "virtio-scsi-pci"
    onboot                 = true
    define_connection_info = false
    ciupgrade              = false
    start                  = true
  }
}

variable "vm_base_config_map" {
  description = "VM configuration options"
  type = object({
    cpu              = optional(string)
    sockets          = optional(number)
    vcpus            = optional(number)
    qemu_os          = optional(string)
    skip_ipv4        = optional(bool)
    skip_ipv6        = optional(bool)
    additional_wait  = optional(number)
    automatic_reboot = optional(bool)
    clone_wait       = optional(number)
  })
  default = {
    cpu       = "x86-64-v3"
    sockets   = 1
    qemu_os   = "l26"
    skip_ipv6 = true
    additional_wait  = 15
    automatic_reboot = true
    clone_wait       = 30
  }
}

variable "cipassword" {
  description = "Cloud-init password"
  type        = string
  sensitive   = true
  default     = null
}

variable "sshkeys" {
  description = "SSH public keys for cloud-init"
  type        = string
  default     = null
}

variable "network" {
  description = "VM network interface configuration"
  type = object({
    bridge    = string
    model     = string
    firewall  = optional(bool)
    link_down = optional(bool)
    tag       = optional(number)
    mtu       = optional(number)
    queues    = optional(number)
    rate      = optional(number)
  })
  default = {
    bridge    = "vmbr0"
    model     = "virtio"
    firewall  = false
    link_down = false
    tag       = 30
  }
}

variable "disk_configurations" {
  description = "VM disk configuration with nested structure"
  type = object({
    scsi = optional(map(object({
      disk = object({
        storage    = string
        size       = string
        format     = optional(string, "raw")
        backup     = optional(bool, true)
        discard    = optional(bool, false)
        emulatessd = optional(bool, false)
        iothread   = optional(bool, false)
        replicate  = optional(bool, false)
        readonly   = optional(bool, false)
        # IO Limits
        iops_r_burst         = optional(number)
        iops_r_burst_length  = optional(number)
        iops_r_concurrent    = optional(number)
        iops_wr_burst        = optional(number)
        iops_wr_burst_length = optional(number)
        iops_wr_concurrent   = optional(number)
        # Bandwidth Limits
        mbps_r_burst       = optional(number)
        mbps_r_concurrent  = optional(number)
        mbps_wr_burst      = optional(number)
        mbps_wr_concurrent = optional(number)
      })
    })), {})
    ide = optional(map(object({
      cdrom = optional(object({
        iso = string
      }))
      cloudinit = optional(object({
        storage = string
      }))
    })), {})
  })
  default = {
    scsi = {
      scsi0 = {
        disk = {
          storage    = "local-lvm"
          size       = "20G"
          format     = "raw"
          backup     = true
          discard    = false
          emulatessd = false
          iothread   = false
          replicate  = false
          readonly   = false
        }
      }
    }
    ide = {
      ide2 = {
        cdrom = {
          iso = "local:iso/ubuntu-24.04.1-live-server-amd64.iso"
        }
      }
      ide3 = {
        cloudinit = {
          storage = "local-lvm"
        }
      }
    }
  }
}

variable "serial" {
  description = "VM serial port configuration"
  type = object({
    id   = number
    type = string
  })
  default = {
    id   = 0
    type = "socket"
  }
}

variable "efidisk" {
  description = "EFI disk configuration"
  type = object({
    efitype = string
    storage = string
  })
  default = {
    efitype = "4m"
    storage = "local-lvm"
  }
}
