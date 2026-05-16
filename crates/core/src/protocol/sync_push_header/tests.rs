use super::*;
use crate::security::EncryptedOp;
use uuid::Uuid;

#[test]
fn source_proof_verifies_signed_diff_payload() {
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let repo_id = Uuid::new_v4();
    let payload = encrypted_payload();

    let header = SyncPushHeader::signed_diff(
        repo_id,
        source_peer,
        VersionVector::new(),
        &payload,
        &source_key,
    )
    .expect("source proof signs");

    assert!(header.validate_source_proof(&payload, true).is_ok());
}

#[test]
fn source_proof_rejects_payload_tamper() {
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let repo_id = Uuid::new_v4();
    let payload = encrypted_payload();
    let mut tampered = payload.clone();
    tampered[0].ciphertext.push(9);

    let header = SyncPushHeader::signed_diff(
        repo_id,
        source_peer,
        VersionVector::new(),
        &payload,
        &source_key,
    )
    .expect("source proof signs");

    assert_eq!(
        header.validate_source_proof(&tampered, true),
        Err(SyncSourceProofError::PayloadDigestMismatch)
    );
}

#[test]
fn source_proof_rejects_wrong_source_key() {
    let claimed_key = IdentityKeyPair::generate();
    let relay_key = IdentityKeyPair::generate();
    let repo_id = Uuid::new_v4();
    let payload = encrypted_payload();
    let mut header = SyncPushHeader::diff(repo_id, claimed_key.peer_id(), VersionVector::new());
    header
        .sign_source(&payload, &relay_key)
        .expect("relay source proof signs");

    assert!(matches!(
        header.validate_source_proof(&payload, true),
        Err(SyncSourceProofError::PeerIdMismatch { .. })
    ));
}

fn encrypted_payload() -> Vec<EncryptedOp> {
    vec![EncryptedOp {
        doc_id: None,
        seq: 1,
        ciphertext: vec![1, 2, 3],
        nonce: vec![0; 12],
    }]
}
