terraform {
  required_providers {
    proxmox = {
      source  = "mrdvince/proxmox"
      version = "0.1.0"
    }
  }
}

provider "proxmox" {
}