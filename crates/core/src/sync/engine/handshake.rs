// crates\core\src\sync\engine
//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::SyncEngine;
use crate::config::SyncMode;
use crate::models::VersionVector;
use crate::models::{PeerId, RepoId};
use crate::security::hashing::sha256_hex;
use crate::security::keypair::verify_signature;
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
        // 1. Verify PeerID (Hash of PubKey)
        // 这里的 12 是截取长度，需与 IdentityKeyPair::peer_id 保持一致
        let hash = sha256_hex(pub_key);
        let derived_id = &hash[0..12];

        if remote_peer_id.as_str() != derived_id {
            return Err(HandshakeError::PeerIdMismatch {
                claimed: remote_peer_id,
                derived: derived_id.to_owned(),
            });
        }

        // 2. Verify Signature
        // Message = "deve-handshake" + peer_id + json(vector)
        // Fix (Deterministic Serialization): Convert HashMap to BTreeMap (sorted keys)
        let sorted_map: std::collections::BTreeMap<_, _> = remote_vector.iter().collect();
        let vec_bytes = serde_json::to_vec(&sorted_map)?;

        let mut msg = Vec::new();
        msg.extend_from_slice(b"deve-handshake");
        msg.extend_from_slice(remote_peer_id.as_str().as_bytes());
        msg.extend_from_slice(&vec_bytes);

        if !verify_signature(pub_key, &msg, signature) {
            return Err(HandshakeError::InvalidSignature);
        }

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
    use crate::ledger::RepoManager;
    use crate::models::VersionVector;
    use crate::models::{PeerId, RepoId};
    use crate::security::IdentityKeyPair;
    use std::sync::Arc;

    fn build_engine() -> anyhow::Result<(tempfile::TempDir, SyncEngine, RepoId)> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let mut repo = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:test:notes"))?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
        let repo = Arc::new(repo);
        let engine = SyncEngine::new(PeerId::new("local"), repo, SyncMode::Auto, None);
        Ok((dir, engine, repo_id))
    }

    fn handshake_signature(remote: &IdentityKeyPair, vector: &VersionVector) -> Vec<u8> {
        let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
        let vec_bytes = serde_json::to_vec(&sorted_map).expect("serialize vector");
        let mut msg = Vec::new();
        msg.extend_from_slice(b"deve-handshake");
        msg.extend_from_slice(remote.peer_id().as_str().as_bytes());
        msg.extend_from_slice(&vec_bytes);
        remote.sign(&msg)
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
