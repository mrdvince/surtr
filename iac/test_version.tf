data "proxmox_version" "example" {}

output "proxmox_version" {
  value = data.proxmox_version.example.version
}

output "proxmox_release" {
  value = data.proxmox_version.example.release
}

output "proxmox_repoid" {
  value = data.proxmox_version.example.repoid
}