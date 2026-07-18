//! Remote projection transport intent protocol types.
//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 09_web_thin_client_ledger#web-edit-intent

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteProjectionProvider {
    WebDav,
    S3,
}

impl RemoteProjectionProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteProjectionProvider::WebDav => "webdav",
            RemoteProjectionProvider::S3 => "s3",
        }
    }
}
