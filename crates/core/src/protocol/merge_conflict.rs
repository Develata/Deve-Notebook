//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeConflictAction {
    AcceptCurrent,
    AcceptIncoming,
    AcceptBoth,
}
