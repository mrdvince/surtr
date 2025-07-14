variable "authentik_url" {
  type        = string
  description = "The base URL for Authentik"
}

variable "client_id" {
  type        = string
  description = "OAuth client ID for Proxmox"
}

variable "client_key" {
  type        = string
  description = "OAuth client secret for Proxmox"
  sensitive   = true
}

variable "admin_password" {
  type        = string
  description = "Admin password for cloud-init in advanced VM example"
  sensitive   = true
  default     = "ChangeMe123!"
}


# variable "template_name" {
#   description = "Name of the template"
#   type        = string
#   default     = null
# }

# variable "os_type" {
#   description = "Type of the OS"
#   type        = string
# }

# variable "target_node" {
#   description = "Node to deploy the VM to"
#   type        = string
#   default     = "mjolnir"
# }

# variable "vmid" {
#   description = "VM ID"
#   type        = number
#   default     = 0
# }

# variable "tags" {
#   description = "Tags for the VM"
#   # default     = null
#   default = "controlplane,talos"
# }

# variable "vm_base_config_map" {
#   description = "Base VM configuration options"
#   type        = map(any)
#   default = {
#     cpu       = "x86-64-v3"
#     skip_ipv6 = true
#   }
# }

# variable "vm_config_map" {
#   type        = map(any)
#   description = "Additional VM configuration options"
#   # bios type can seabios or ovmf
#   default = {
#     bios                   = "ovmf"
#     boot                   = "c"
#     bootdisk               = "ide2"
#     cores                  = 2
#     define_connection_info = false
#     machine                = "q35"
#     memory                 = 4096
#     onboot                 = true
#     scsihw                 = "virtio-scsi-pci"
#     balloon                = 4096
#   }
# }

# variable "sshkeys" {
#   description = "SSH keys used in the VM"
#   default     = file("~/.ssh/devkey.pub")
# }

# variable "cipassword" {
#   description = "Password for the VM"
#   type        = string
#   default     = null
# }

# variable "network" {
#   description = "VM Network configuration"
#   type        = map(any)
#   default = {
#     bridge    = "vmbr0"
#     firewall  = false
#     link_down = false
#     model     = "virtio"
#     tag       = 30
#   }
# }

# variable "serial" {
#   description = "VM Serial configuration"
#   type        = map(any)
#   default = {
#     id   = 0
#     type = "socket"
#   }
# }

# variable "efidisk" {
#   description = "EFI Disk config, can be null"
#   type        = map(any)
#   default = {
#     efitype = "4m"
#     storage = "local-lvm"
#   }
# }

# variable "instances" {
#   description = "VM instances"
#   type = list(object({
#     vmname   = string,
#     vmid     = number,
#     ipconfig = string,
#     macaddr  = string
#   }))
#   default = [{
#     vmname   = "athos-01"
#     vmid     = 9001
#     ipconfig = "ip=dhcp"
#     macaddr  = "ba:88:cb:76:75:d6"
#   }]
# }

# variable "disk_configurations" {
#   type = map(any)
#   default = {
#     scsi = {
#       scsi0 = { disk = {
#         storage    = "local-lvm"
#         backup     = true
#         discard    = false
#         emulatessd = false
#         format     = "raw"
#         iothread   = false
#         readonly   = false
#         replicate  = false
#         size = "128G" }
#       }
#     }

#     ide = {
#       ide2 = {
#         cdrom = { iso = "local:iso/ubuntu-24.04.1-live-server-amd64.iso" }
#       }
#     }
#   }
# }