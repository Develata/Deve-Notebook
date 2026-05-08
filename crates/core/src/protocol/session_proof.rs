//! Sync session proof protocol entity.
//! plan_ref:
//!   - 05_network#web-ws-runtime

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionProof {
    pub signature: Vec<u8>,
}

impl SessionProof {
    pub fn new(signature: Vec<u8>) -> Self {
        Self { signature }
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::SessionProof;
    use crate::models::{PeerId, VersionVector};
    use crate::protocol::ClientMessage;
    use crate::protocol::frame::{ClientFrame, decode_client_json};

    #[test]
    fn sync_hello_uses_session_proof_fields_in_versioned_json() {
        let peer_id = PeerId::new("peer-a");
        let repo_id = uuid::Uuid::new_v4();
        let message = ClientMessage::SyncHello {
            peer_id: peer_id.clone(),
            peer_pubkey: vec![1, 2, 3],
            session_proof: SessionProof::new(vec![4, 5, 6]),
            vector: VersionVector::new(),
            repo_id,
            scope_nonce: 7,
        };
        let text = serde_json::to_string(&ClientFrame::current(message)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(
            value["message"]["SyncHello"]["peer_pubkey"],
            serde_json::json!([1, 2, 3])
        );
        assert_eq!(
            value["message"]["SyncHello"]["session_proof"]["signature"],
            serde_json::json!([4, 5, 6])
        );
        assert!(value["message"]["SyncHello"].get("pub_key").is_none());

        match decode_client_json(&text).unwrap() {
            ClientMessage::SyncHello {
                peer_id: decoded_peer,
                peer_pubkey,
                session_proof,
                repo_id: decoded_repo,
                scope_nonce,
                ..
            } => {
                assert_eq!(decoded_peer, peer_id);
                assert_eq!(peer_pubkey, vec![1, 2, 3]);
                assert_eq!(session_proof.signature(), &[4, 5, 6]);
                assert_eq!(decoded_repo, repo_id);
                assert_eq!(scope_nonce, 7);
            }
            other => panic!("expected SyncHello, got {other:?}"),
        }
    }
}
