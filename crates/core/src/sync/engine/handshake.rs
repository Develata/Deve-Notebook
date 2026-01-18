// crates\core\src\sync\engine
use super::SyncEngine;
use crate::config::SyncMode;
use crate::models::PeerId;
use crate::security::hashing::sha256_hex;
use crate::security::keypair::verify_signature;
use crate::sync::protocol::{self, HandshakeResult};
use crate::sync::vector::VersionVector;
use anyhow::{Result, anyhow};

impl SyncEngine {
    /// 计算与远端 Peer 的差异 (Internal)
    pub fn compute_diff(
        &self,
        remote_vector: &VersionVector,
    ) -> (Vec<protocol::SyncRequest>, Vec<protocol::SyncRequest>) {
        // TODO: Pass actual RepoId
        protocol::compute_diff_requests(&self.version_vector, remote_vector, uuid::Uuid::nil())
    }

    /// 执行完整的握手流程 (Secure)
    ///
    /// **验证步骤**:
    /// 1. 验证 PeerID 是否由 PubKey 改写 (Hash check)。
    /// 2. 验证 Signature 是否有效 (防止中间人篡改 Vector)。
    pub fn handshake(
        &mut self,
        remote_peer_id: PeerId,
        pub_key: &[u8],
        signature: &[u8],
        remote_vector: VersionVector,
    ) -> Result<HandshakeResult> {
        // 1. Verify PeerID (Hash of PubKey)
        // 这里的 12 是截取长度，需与 IdentityKeyPair::peer_id 保持一致
        let hash = sha256_hex(pub_key);
        let derived_id = &hash[0..12];

        if remote_peer_id.as_str() != derived_id {
            return Err(anyhow!(
                "PeerID mismatch: claimed {}, derived {}",
                remote_peer_id,
                derived_id
            ));
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
            return Err(anyhow!("Invalid Handshake Signature"));
        }

        // 3. Compute Diff
        let (to_send, to_request) = self.compute_diff(&remote_vector);

        Ok(HandshakeResult {
            to_send,
            to_request,
            auto_apply: self.sync_mode == SyncMode::Auto,
        })
    }
}
