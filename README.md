# An OpenTofu/Terraform Provider for Proxmox

> This is a learning project and experimentantion to see what is required to write providers in a different language essentially if it can be done and how it would look like.

I use proxmox instances and have been using the Telmate provider for a while
But i mostly used the `proxmox_vm_qemu` resource, for other resources I resulted to [stuff like this](https://mrdvince.me/proxmox-oidc-integration-and-terragrunt-hooks-day-36/)

## Requirements

- Rust (1.87.0)
- OpenTofu (tested on v1.9.1)
- TLS certificates (generated with mkcert)

## Building

```bash
# Clone the repository
git clone <your-repo-url>
cd surtr

# Generate TLS certificates (required for gRPC)
mkcert -install
cd certs
mkcert localhost 127.0.0.1 ::1
cd ..

# Build the provider
cargo build --release
```

## Installation

### Local Development with OpenTofu

Create a `.terraformrc` file in your home directory:

```
provider_installation {
  dev_overrides {
    "mrdvince/proxmox" = "/path/to/surtr/target/release"
  }
  direct {}
}
```

## Usage

```
terraform {
  required_providers {
    proxmox = {
      source  = "mrdvince/proxmox"
      version = "0.1.0"
    }
  }
}

provider "proxmox" {
  endpoint  = "https://your-proxmox-server:8006"
  api_token = "user@realm!tokenid=your-token-secret"
  insecure  = false  # Set to true to skip TLS verification
}

data "proxmox_version" "pve" {}

output "proxmox_version" {
  value = data.proxmox_version.pve.version
}
```

### Running with OpenTofu

```bash
# Initialize the provider
tofu init

# Plan changes
tofu plan

# Apply changes
tofu apply
```

## Provider Configuration

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `endpoint` | String | Yes | The Proxmox API endpoint URL (e.g., `https://pve.example.com:8006`) |
| `api_token` | String | Yes | API token in format `user@realm!tokenid=secret` |
| `insecure` | Boolean | No | Skip TLS certificate verification (default: `false`) |

Planned to add:
- [ ] Resource: `proxmox_vm_qemu` - to manage VMs
- [ ] Resource: `proxmox_realm` - manage authentication realms (OIDC, etc.)
- [ ] Other things I use as I go

## References
- https://pve.proxmox.com/pve-docs/api-viewer/
- https://github.com/opentofu/opentofu/tree/main/docs/plugin-protocol
- https://github.com/hashicorp/terraform-plugin-go
- https://github.com/hashicorp/terraform-plugin-framework  
- https://github.com/hashicorp/terraform-provider-scaffolding-framework
- https://github.com/hashicorp/terraform-provider-aws
- https://developer.hashicorp.com/terraform/plugin/framework
- https://developer.hashicorp.com/terraform/tutorials/providers-plugin-framework
- https://developer.hashicorp.com/terraform/plugin/terraform-plugin-protocol


## Written with Claude

This provider was written with the help of Claude. But this wasn't "vibe coded" per se (albeit Claude knows more rust than I do).

Through this project, I learned:
- How the Terraform Plugin Protocol actually works under the hood
- gRPC
- Refresher on my already "rusty" rust knowhow
- How/what it's like to "pair" with the LLMs, basiaclly turned reviewer for most of the code (not auto-accepting).

"Can you really learn by building with an AI?"

Yes.