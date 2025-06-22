terraform {
  required_providers {
    proxmox = {
      source  = "mrdvince/proxmox"
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