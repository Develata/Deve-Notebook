//! plan_ref:
//!   - 07_network#repo-scoped-handshake
//!
//! Shared SyncHello proof transcript construction and verification.

use crate::models::{PeerId, VersionVector};
use crate::security::IdentityKeyPair;
use crate::security::hashing::sha256_hex;
use crate::security::keypair::verify_signature;
use thiserror::Error;

const HANDSHAKE_DOMAIN: &[u8] = b"deve-handshake";

#[derive(Debug, Error)]
pub enum SyncHelloProofError {
    #[error("PeerID mismatch: claimed {claimed}, derived {derived}")]
    PeerIdMismatch { claimed: PeerId, derived: String },

    #[error("Invalid Handshake Signature")]
    InvalidSignature,

    #[error("Failed to encode handshake vector: {0}")]
    VectorSerialization(#[from] serde_json::Error),
}

impl SyncHelloProofError {
    pub const fn is_peer_auth_failure(&self) -> bool {
        matches!(
            self,
            SyncHelloProofError::PeerIdMismatch { .. } | SyncHelloProofError::InvalidSignature
        )
    }
}

pub fn sync_hello_transcript(
    peer_id: &PeerId,
    vector: &VersionVector,
) -> Result<Vec<u8>, serde_json::Error> {
    let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
    let vector_bytes = serde_json::to_vec(&sorted_map)?;
    let mut transcript =
        Vec::with_capacity(HANDSHAKE_DOMAIN.len() + peer_id.as_str().len() + vector_bytes.len());
    transcript.extend_from_slice(HANDSHAKE_DOMAIN);
    transcript.extend_from_slice(peer_id.as_str().as_bytes());
    transcript.extend_from_slice(&vector_bytes);
    Ok(transcript)
}

pub fn sign_sync_hello(
    identity: &IdentityKeyPair,
    vector: &VersionVector,
) -> Result<Vec<u8>, serde_json::Error> {
    let transcript = sync_hello_transcript(&identity.peer_id(), vector)?;
    Ok(identity.sign(&transcript))
}

pub fn verify_sync_hello_proof(
    peer_id: &PeerId,
    pub_key: &[u8],
    signature: &[u8],
    vector: &VersionVector,
) -> Result<(), SyncHelloProofError> {
    let hash = sha256_hex(pub_key);
    let derived_id = &hash[0..12];

    if peer_id.as_str() != derived_id {
        return Err(SyncHelloProofError::PeerIdMismatch {
            claimed: peer_id.clone(),
            derived: derived_id.to_owned(),
        });
    }

    let transcript = sync_hello_transcript(peer_id, vector)?;
    if !verify_signature(pub_key, &transcript, signature) {
        return Err(SyncHelloProofError::InvalidSignature);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{sign_sync_hello, sync_hello_transcript, verify_sync_hello_proof};
    use crate::models::{PeerId, VersionVector};
    use crate::security::IdentityKeyPair;

    #[test]
    fn sync_hello_proof_round_trips_with_canonical_vector() -> anyhow::Result<()> {
        let identity = IdentityKeyPair::generate();
        let mut vector = VersionVector::new();
        vector.update(PeerId::new("z-peer"), 2);
        vector.update(PeerId::new("a-peer"), 1);

        let signature = sign_sync_hello(&identity, &vector)?;

        verify_sync_hello_proof(
            &identity.peer_id(),
            &identity.public_key_bytes(),
            &signature,
            &vector,
        )?;
        Ok(())
    }

    #[test]
    fn sync_hello_transcript_is_domain_separated() -> anyhow::Result<()> {
        let identity = IdentityKeyPair::generate();
        let transcript = sync_hello_transcript(&identity.peer_id(), &VersionVector::new())?;

        assert!(transcript.starts_with(b"deve-handshake"));
        assert!(
            transcript
                .windows(identity.peer_id().as_str().len())
                .any(|window| { window == identity.peer_id().as_str().as_bytes() })
        );
        Ok(())
    }
}
