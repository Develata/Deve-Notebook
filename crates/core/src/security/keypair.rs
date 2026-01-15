//! # 密钥对管理 (KeyPair Management)
//!
//! **功能**:
//! 管理节点的身份密钥 (Identity Key)。基于 Ed25519 椭圆曲线签名算法。
//!
//! **设计**:
//! - `IdentityKeyPair`: 包含公钥和私钥，用于签名握手消息。
//! - `sign`: 对消息进行签名。
//! - `verify`: 验证签名是否来自该公钥。

use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use rand::rngs::OsRng;
use rand::RngCore; // Import RngCore for fill_bytes
use serde::{Deserialize, Serialize};
use crate::models::PeerId;
use super::hashing::sha256_hex;

/// 身份密钥对 (包含私钥)
/// 
/// **注意**: 严禁将私钥泄露给网络或其他模块。
pub struct IdentityKeyPair {
    signing_key: SigningKey,
}

impl IdentityKeyPair {
    /// 生成新的随机密钥对
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut bytes = [0u8; 32];
        csprng.fill_bytes(&mut bytes);
        let signing_key = SigningKey::from_bytes(&bytes);
        Self { signing_key }
    }

    /// 获取公钥字节
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// 获取对应的 PeerID (Hash of Public Key)
    pub fn peer_id(&self) -> PeerId {
        let pub_bytes = self.public_key_bytes();
        let hash = sha256_hex(&pub_bytes);
        // 取 Hash 的前 12 位作为简短 ID (类似 Git SHA)
        PeerId::new(&hash[0..12])
    }

    /// 对消息进行签名
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }
}

/// 验证签名
///
/// **参数**:
/// - `pub_key_bytes`: 对方的公钥 (32 bytes)
/// - `message`: 原始消息
/// - `signature_bytes`: 给定的签名 (64 bytes)
pub fn verify_signature(pub_key_bytes: &[u8], message: &[u8], signature_bytes: &[u8]) -> bool {
    if let Ok(verifying_key) = VerifyingKey::from_bytes(pub_key_bytes.try_into().unwrap_or(&[0; 32])) {
        // Signature::from_bytes returns Signature directly (not Result) for fixed array
        if let Ok(sig_arr) = signature_bytes.try_into() {
             let signature = Signature::from_bytes(sig_arr);
             return verifying_key.verify(message, &signature).is_ok();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_verify() {
        let keypair = IdentityKeyPair::generate();
        let msg = b"handshake challenge";
        let sig = keypair.sign(msg);
        
        // 验证合法的
        assert!(verify_signature(&keypair.public_key_bytes(), msg, &sig));
        
        // 验证篡改的
        assert!(!verify_signature(&keypair.public_key_bytes(), b"tampered", &sig));
    }

    #[test]
    fn test_peer_id_integrity() {
        let keypair = IdentityKeyPair::generate();
        let pid = keypair.peer_id();
        assert_eq!(pid.as_str().len(), 12);
    }
}
