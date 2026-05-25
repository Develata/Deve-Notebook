//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 07_network#web-ws-runtime
//!
//! Plaintext sync payload envelope header.

use crate::models::{PeerId, RepoId, VersionVector};
use crate::security::IdentityKeyPair;
use crate::security::hashing::sha256_hex;
use crate::security::keypair::verify_signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

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
    #[serde(default)]
    pub source_proof: Option<SyncSourceProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncSourceProof {
    pub source_pubkey: Vec<u8>,
    pub payload_digest: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyncSourceProofError {
    #[error("missing source proof for indirect sync payload")]
    Missing,
    #[error("source proof payload digest mismatch")]
    PayloadDigestMismatch,
    #[error("source proof peer mismatch: claimed {claimed}, derived {derived}")]
    PeerIdMismatch { claimed: PeerId, derived: String },
    #[error("invalid source proof public key length: {0}")]
    InvalidPubkeyLength(usize),
    #[error("invalid source proof signature")]
    InvalidSignature,
    #[error("failed to encode source proof message: {0}")]
    MessageEncode(String),
    #[error("failed to encode encrypted payload digest input: {0}")]
    PayloadEncode(String),
}

impl SyncPushHeader {
    pub fn diff(repo_id: RepoId, peer_id: PeerId, vector: VersionVector) -> Self {
        Self {
            repo_id,
            peer_id,
            vector,
            payload_kind: SyncPayloadKind::Diff,
            source_proof: None,
        }
    }

    pub fn signed_diff(
        repo_id: RepoId,
        peer_id: PeerId,
        vector: VersionVector,
        payload: &[crate::security::EncryptedOp],
        source_key: &IdentityKeyPair,
    ) -> Result<Self, SyncSourceProofError> {
        let mut header = Self::diff(repo_id, peer_id, vector);
        header.sign_source(payload, source_key)?;
        Ok(header)
    }

    pub fn sign_source(
        &mut self,
        payload: &[crate::security::EncryptedOp],
        source_key: &IdentityKeyPair,
    ) -> Result<(), SyncSourceProofError> {
        self.source_proof = Some(SyncSourceProof::sign(
            self.repo_id,
            &self.peer_id,
            &self.vector,
            self.payload_kind.clone(),
            payload,
            source_key,
        )?);
        Ok(())
    }

    pub fn validate_source_proof(
        &self,
        payload: &[crate::security::EncryptedOp],
        required: bool,
    ) -> Result<(), SyncSourceProofError> {
        match &self.source_proof {
            Some(proof) => proof.verify(
                self.repo_id,
                &self.peer_id,
                &self.vector,
                self.payload_kind.clone(),
                payload,
            ),
            None if required => Err(SyncSourceProofError::Missing),
            None => Ok(()),
        }
    }
}

impl SyncSourceProof {
    pub fn sign(
        repo_id: RepoId,
        source_peer_id: &PeerId,
        vector: &VersionVector,
        payload_kind: SyncPayloadKind,
        payload: &[crate::security::EncryptedOp],
        source_key: &IdentityKeyPair,
    ) -> Result<Self, SyncSourceProofError> {
        let payload_digest = encrypted_payload_digest(payload)?;
        let message = source_proof_message(
            repo_id,
            source_peer_id,
            vector,
            payload_kind,
            &payload_digest,
        )?;
        Ok(Self {
            source_pubkey: source_key.public_key_bytes().to_vec(),
            payload_digest,
            signature: source_key.sign(&message),
        })
    }

    pub fn verify(
        &self,
        repo_id: RepoId,
        source_peer_id: &PeerId,
        vector: &VersionVector,
        payload_kind: SyncPayloadKind,
        payload: &[crate::security::EncryptedOp],
    ) -> Result<(), SyncSourceProofError> {
        let payload_digest = encrypted_payload_digest(payload)?;
        if self.payload_digest != payload_digest {
            return Err(SyncSourceProofError::PayloadDigestMismatch);
        }

        let derived = source_peer_id_from_pubkey(&self.source_pubkey)?;
        if &derived != source_peer_id {
            return Err(SyncSourceProofError::PeerIdMismatch {
                claimed: source_peer_id.clone(),
                derived: derived.to_string(),
            });
        }

        let message = source_proof_message(
            repo_id,
            source_peer_id,
            vector,
            payload_kind,
            &payload_digest,
        )?;
        if !verify_signature(&self.source_pubkey, &message, &self.signature) {
            return Err(SyncSourceProofError::InvalidSignature);
        }
        Ok(())
    }
}

fn encrypted_payload_digest(
    payload: &[crate::security::EncryptedOp],
) -> Result<Vec<u8>, SyncSourceProofError> {
    let bytes = bincode::serialize(payload)
        .map_err(|err| SyncSourceProofError::PayloadEncode(err.to_string()))?;
    Ok(Sha256::digest(bytes).to_vec())
}

fn source_peer_id_from_pubkey(pubkey: &[u8]) -> Result<PeerId, SyncSourceProofError> {
    if pubkey.len() != 32 {
        return Err(SyncSourceProofError::InvalidPubkeyLength(pubkey.len()));
    }
    let hash = sha256_hex(pubkey);
    Ok(PeerId::new(&hash[0..12]))
}

fn source_proof_message(
    repo_id: RepoId,
    source_peer_id: &PeerId,
    vector: &VersionVector,
    payload_kind: SyncPayloadKind,
    payload_digest: &[u8],
) -> Result<Vec<u8>, SyncSourceProofError> {
    let mut vector_entries: Vec<(String, u64)> = vector
        .iter()
        .map(|(peer, seq)| (peer.as_str().to_string(), *seq))
        .collect();
    vector_entries.sort_by(|left, right| left.0.cmp(&right.0));
    let payload_kind = match payload_kind {
        SyncPayloadKind::Diff => "diff",
        SyncPayloadKind::Snapshot => "snapshot",
    };
    let message = SourceProofMessage {
        domain: "deve-sync-source-v1",
        repo_id: repo_id.to_string(),
        source_peer_id: source_peer_id.as_str().to_string(),
        vector: vector_entries,
        payload_kind,
        payload_digest,
    };
    serde_json::to_vec(&message).map_err(|err| SyncSourceProofError::MessageEncode(err.to_string()))
}

#[derive(Serialize)]
struct SourceProofMessage<'a> {
    domain: &'static str,
    repo_id: String,
    source_peer_id: String,
    vector: Vec<(String, u64)>,
    payload_kind: &'static str,
    payload_digest: &'a [u8],
}

#[cfg(test)]
mod tests;
