//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!

pub fn new_writer_session_nonce() -> u64 {
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let mut nonce = [0u8; 8];
    nonce.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(nonce)
}

pub fn derive_writer_client_id(peer_id: &str, session_nonce: u64) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in peer_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for byte in session_nonce.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::derive_writer_client_id;

    #[test]
    fn stable_for_same_peer_and_session() {
        assert_eq!(
            derive_writer_client_id("browser-peer-a", 1),
            derive_writer_client_id("browser-peer-a", 1)
        );
    }

    #[test]
    fn differs_for_different_peers_or_sessions() {
        assert_ne!(
            derive_writer_client_id("browser-peer-a", 1),
            derive_writer_client_id("browser-peer-b", 1)
        );
        assert_ne!(
            derive_writer_client_id("browser-peer-a", 1),
            derive_writer_client_id("browser-peer-a", 2)
        );
    }
}
