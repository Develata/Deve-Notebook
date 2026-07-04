//! Remote projection transport intent protocol types.
//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 09_web_thin_client_ledger#web-edit-intent

use serde::{Deserialize, Serialize};

pub const REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL: &str =
    "remote-projection-provider-io-ready-false; provider_io_ready=false";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteProjectionProvider {
    WebDav,
    S3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteProjectionDirection {
    Push,
    Pull,
}

impl RemoteProjectionProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteProjectionProvider::WebDav => "webdav",
            RemoteProjectionProvider::S3 => "s3",
        }
    }
}

impl RemoteProjectionDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteProjectionDirection::Push => "push",
            RemoteProjectionDirection::Pull => "pull",
        }
    }
}
