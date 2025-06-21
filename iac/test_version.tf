terraform {
  required_providers {
    proxmox = {
      source = "mrdvince/proxmox"
      version = "0.1.0"
    }
  }
}

provider "proxmox" {
  # endpoint, api_token, and insecure will be read from:
  # - PROXMOX_ENDPOINT
  # - PROXMOX_API_TOKEN  
  # - PROXMOX_INSECURE
}

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