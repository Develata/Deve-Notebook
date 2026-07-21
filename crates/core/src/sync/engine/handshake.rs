// crates\core\src\sync\engine
//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::SyncEngine;
use crate::config::SyncMode;
use crate::models::VersionVector;
use crate::models::{PeerId, RepoId};
use crate::sync::handshake_proof::{SyncHelloProofError, verify_sync_hello_proof};
use crate::sync::protocol::{self, HandshakeResult};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("PeerID mismatch: claimed {claimed}, derived {derived}")]
    PeerIdMismatch { claimed: PeerId, derived: String },

    #[error("Invalid Handshake Signature")]
    InvalidSignature,

    #[error("Failed to encode handshake vector: {0}")]
    VectorSerialization(#[from] serde_json::Error),
}

impl HandshakeError {
    pub const fn is_peer_auth_failure(&self) -> bool {
        matches!(
            self,
            HandshakeError::PeerIdMismatch { .. } | HandshakeError::InvalidSignature
        )
    }
}

impl From<SyncHelloProofError> for HandshakeError {
    fn from(err: SyncHelloProofError) -> Self {
        match err {
            SyncHelloProofError::PeerIdMismatch { claimed, derived } => {
                HandshakeError::PeerIdMismatch { claimed, derived }
            }
            SyncHelloProofError::InvalidSignature => HandshakeError::InvalidSignature,
            SyncHelloProofError::VectorSerialization(err) => {
                HandshakeError::VectorSerialization(err)
            }
        }
    }
}

impl SyncEngine {
    /// 计算与远端 Peer 的差异 (Internal)
    ///
    /// 仓库路由必须显式携带 `repo_id`，否则多仓库同步会退化为错误的空仓库占位路由。
    pub fn compute_diff(
        &self,
        remote_vector: &VersionVector,
        repo_id: RepoId,
    ) -> (
        Vec<protocol::SyncRequest>,
        Vec<protocol::SyncRequest>,
        Vec<protocol::SyncSnapshotRequest>,
    ) {
        protocol::compute_diff_requests(&self.version_vector, remote_vector, repo_id)
    }

    /// 执行完整的握手流程 (Secure)
    ///
    /// **验证步骤**:
    /// 1. 验证 PeerID 是否由 PubKey 改写 (Hash check)。
    /// 2. 验证 Signature 是否有效 (防止中间人篡改 Vector)。
    pub fn handshake(
        &mut self,
        repo_id: RepoId,
        remote_peer_id: PeerId,
        pub_key: &[u8],
        signature: &[u8],
        remote_vector: VersionVector,
    ) -> Result<HandshakeResult, HandshakeError> {
        verify_sync_hello_proof(&remote_peer_id, pub_key, signature, &remote_vector)?;

        let mut remote_vector = remote_vector;
        remote_vector.normalize();

        // 3. Compute Diff
        let (to_send, to_request, snapshot_requests) = self.compute_diff(&remote_vector, repo_id);

        Ok(HandshakeResult {
            to_send,
            to_request,
            snapshot_requests,
            auto_apply: self.sync_mode == SyncMode::Auto,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{HandshakeError, SyncEngine};
    use crate::config::SyncMode;
    use crate::models::VersionVector;
    use crate::models::{PeerId, RepoId};
    use crate::security::IdentityKeyPair;
    use crate::sync::handshake_proof::sign_sync_hello;
    use std::sync::Arc;

    fn build_engine() -> anyhow::Result<(tempfile::TempDir, SyncEngine, RepoId)> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let (repo, repo_id) = crate::test_support::init_cataloged_repo_with_url(
            &ledger,
            &projection_base,
            "urn:test:notes",
        )?;
        let repo = Arc::new(repo);
        let engine = SyncEngine::new(PeerId::new("local"), repo, SyncMode::Auto, None);
        Ok((dir, engine, repo_id))
    }

    fn handshake_signature(remote: &IdentityKeyPair, vector: &VersionVector) -> Vec<u8> {
        sign_sync_hello(remote, vector).expect("serialize vector")
    }

    #[test]
    fn handshake_reports_peer_id_mismatch_as_typed_auth_error() -> anyhow::Result<()> {
        let (_dir, mut engine, repo_id) = build_engine()?;
        let remote = IdentityKeyPair::generate();
        let other = IdentityKeyPair::generate();
        let vector = VersionVector::new();
        let other_pubkey = other.public_key_bytes();

        let err = engine
            .handshake(
                repo_id,
                remote.peer_id(),
                &other_pubkey,
                &handshake_signature(&remote, &vector),
                vector,
            )
            .expect_err("mismatched pubkey must fail");

        assert!(matches!(err, HandshakeError::PeerIdMismatch { .. }));
        assert!(err.is_peer_auth_failure());
        Ok(())
    }

    #[test]
    fn handshake_reports_invalid_signature_as_typed_auth_error() -> anyhow::Result<()> {
        let (_dir, mut engine, repo_id) = build_engine()?;
        let remote = IdentityKeyPair::generate();
        let pubkey = remote.public_key_bytes();

        let err = engine
            .handshake(
                repo_id,
                remote.peer_id(),
                &pubkey,
                &[0; 64],
                VersionVector::new(),
            )
            .expect_err("invalid signature must fail");

        assert!(matches!(err, HandshakeError::InvalidSignature));
        assert!(err.is_peer_auth_failure());
        Ok(())
    }
}
