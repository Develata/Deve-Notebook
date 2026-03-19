use super::buffer_encrypted_ops_until_key;
use deve_core::security::{EncryptedOp, RepoKey};
use std::sync::Mutex;

fn sample_encrypted_op(seq: u64) -> EncryptedOp {
    EncryptedOp {
        doc_id: None,
        seq,
        ciphertext: vec![1, 2, 3],
        nonce: vec![4; 12],
    }
}

#[test]
fn sync_push_without_repo_key_is_buffered() {
    let buffered = Mutex::new(Vec::new());
    let ops = vec![sample_encrypted_op(7), sample_encrypted_op(8)];

    let key = buffer_encrypted_ops_until_key(&buffered, None, &ops);

    assert!(key.is_none());
    let buffered = buffered.lock().unwrap();
    assert_eq!(buffered.len(), 2);
    assert_eq!(buffered[0].seq, 7);
    assert_eq!(buffered[1].seq, 8);
}

#[test]
fn sync_push_with_repo_key_skips_buffering() {
    let buffered = Mutex::new(vec![sample_encrypted_op(3)]);
    let expected = RepoKey::generate();
    let ops = vec![sample_encrypted_op(7)];

    let key = buffer_encrypted_ops_until_key(&buffered, Some(expected.clone()), &ops);

    assert!(key.is_some());
    assert_eq!(buffered.lock().unwrap().len(), 1);
}
