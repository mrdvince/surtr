//! Proxmox API client implementation

pub mod access;
pub mod client;
pub mod common;
pub mod error;
pub mod pool;
pub mod response;
pub mod version;

#[cfg(test)]
mod test_helpers;

pub use access::AccessApi;
pub use client::*;
pub use common::{
    deserialize_proxmox_bool_option, ApiErrorDetails, ApiErrorResponse, ApiQueryParams,
    ApiResponse, PaginationParams, ProxmoxApiResource, ProxmoxBool,
};
pub use error::*;
