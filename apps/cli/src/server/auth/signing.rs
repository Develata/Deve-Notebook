//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!
//! Small server-side signing primitives for local auth adjuncts.

use deve_core::security::hashing::sha256_bytes;

pub(crate) fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    hex_encode(&hmac_sha256(key, message))
}

pub(crate) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (left, right) in left.iter().zip(right) {
        diff |= left ^ right;
    }
    diff == 0
}

pub(crate) fn is_hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_LEN: usize = 64;
    let mut key_block = [0_u8; BLOCK_LEN];
    if key.len() > BLOCK_LEN {
        key_block[..32].copy_from_slice(&sha256_bytes(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut outer_pad = [0x5c_u8; BLOCK_LEN];
    let mut inner_pad = [0x36_u8; BLOCK_LEN];
    for index in 0..BLOCK_LEN {
        outer_pad[index] ^= key_block[index];
        inner_pad[index] ^= key_block[index];
    }

    let mut inner = Vec::with_capacity(BLOCK_LEN + message.len());
    inner.extend_from_slice(&inner_pad);
    inner.extend_from_slice(message);
    let inner_hash = sha256_bytes(&inner);

    let mut outer = Vec::with_capacity(BLOCK_LEN + inner_hash.len());
    outer.extend_from_slice(&outer_pad);
    outer.extend_from_slice(&inner_hash);
    sha256_bytes(&outer)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hmac_sha256_hex;

    #[test]
    fn hmac_sha256_matches_rfc_4231_test_vector() {
        let key = [0x0b_u8; 20];
        let digest = hmac_sha256_hex(&key, b"Hi There");

        assert_eq!(
            digest,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }
}
