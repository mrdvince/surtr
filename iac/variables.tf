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
