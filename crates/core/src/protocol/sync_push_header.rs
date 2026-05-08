//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 05_network#web-ws-runtime
//!
//! Plaintext sync payload envelope header.

use crate::models::{PeerId, RepoId, VersionVector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPayloadKind {
    Diff,
    Snapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncPushHeader {
    pub repo_id: RepoId,
    pub peer_id: PeerId,
    pub vector: VersionVector,
    pub payload_kind: SyncPayloadKind,
}

impl SyncPushHeader {
    pub fn diff(repo_id: RepoId, peer_id: PeerId, vector: VersionVector) -> Self {
        Self {
            repo_id,
            peer_id,
            vector,
            payload_kind: SyncPayloadKind::Diff,
        }
    }
}
