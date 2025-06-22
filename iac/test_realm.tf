resource "proxmox_realm" "minimal" {
  realm      = "test-minimal"
  type       = "openid"
  issuer_url = "https://auth.example.com"
  client_id  = "proxmox-test"
  client_key = "super-secret-key"
}

resource "proxmox_realm" "full" {
  realm             = "test-full"
  type              = "openid"
  issuer_url        = "https://auth.example.com"
  client_id         = "proxmox-test"
  client_key        = "super-secret-key"
  username_claim    = "email"
  autocreate        = true
  default           = false
  comment           = "Updated full realm with all options"
  groups_overwrite  = true
  groups_autocreate = false
}

resource "proxmox_realm" "authentik" {
  realm             = "test-authentik"
  type              = "openid"
  issuer_url        = "https://${var.authentik_url}/application/o/proxmox-test/"
  client_id         = var.client_id
  client_key        = var.client_key
  username_claim    = "username"
  autocreate        = true
  default           = false
  comment           = "Realm with authentik test"
  groups_overwrite  = true
  groups_autocreate = true
}


output "minimal_realm" {
  value = proxmox_realm.minimal.realm
}

output "full_realm" {
  value = proxmox_realm.full.realm
}

output "authentik_issuer_url" {
  value = proxmox_realm.authentik.issuer_url
}