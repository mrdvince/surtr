resource "proxmox_realm" "test_oidc" {
  realm        = "test-oidc"
  type         = "openid"
  issuer_url   = "https://auth.example.com"
  client_id    = "proxmox-test"
  client_key   = "super-secret-key"
  
  username_claim = "email"
  autocreate     = true
  default        = false
  comment        = "Updated OIDC realm"
}

output "realm_name" {
  value = proxmox_realm.test_oidc.realm
}

output "realm_type" {
  value = proxmox_realm.test_oidc.type
}

output "issuer_url" {
  value = proxmox_realm.test_oidc.issuer_url
}