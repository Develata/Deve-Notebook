// crates\core\src\security
//! # 加密模块 (Cipher)
//!
//! **功能**:
//! 提供数据加密原语，基于 AES-256-GCM (Authenticated Encryption)。
//! 用于实现 "Envelope Pattern" 中的 Paylaod 加密。
//!
//! **设计**:
//! - `RepoKey`: 32 字节对称密钥。
//! - `EncryptedOp`: 加密后的操作载荷结构。

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce
};
use serde::{Deserialize, Serialize};
use crate::models::{DocId, LedgerEntry};

/// 仓库密钥 (AES-256)
/// 
/// **用途**:
/// 加密同步过程中传输的 `LedgerEntry`。
/// 只有拥有此密钥的 Peer 才能解密数据。
#[derive(Clone)]
pub struct RepoKey {
    key_bytes: [u8; 32],
    cipher: Aes256Gcm,
}

impl RepoKey {
    /// 生成新的随机密钥
    pub fn generate() -> Self {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let key_bytes: [u8; 32] = key.into();
        Self {
            key_bytes,
            cipher: Aes256Gcm::new(&key),
        }
    }

    /// 从现有字节加载密钥
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 32 {
            return None;
        }
        let key_bytes: [u8; 32] = bytes.try_into().ok()?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        Some(Self {
            key_bytes,
            cipher: Aes256Gcm::new(key),
        })
    }

    /// 导出密钥字节 (用于持久化)
    /// 
    /// **安全警告**: 导出的字节应当安全存储，避免泄露
    pub fn to_bytes(&self) -> [u8; 32] {
        self.key_bytes
    }

    /// 加密 LedgerEntry
    pub fn encrypt(&self, entry: &LedgerEntry) -> anyhow::Result<EncryptedOp> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
        let plaintext = serde_json::to_vec(entry)?;
        
        let ciphertext = self.cipher.encrypt(&nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(EncryptedOp {
            doc_id: entry.doc_id,
            seq: 0, // 这里的 Seq 需由外部 Sync 逻辑填充
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    /// 解密
    pub fn decrypt(&self, enc: &EncryptedOp) -> anyhow::Result<LedgerEntry> {
        let nonce = Nonce::from_slice(&enc.nonce);
        let plaintext = self.cipher.decrypt(nonce, enc.ciphertext.as_ref())
            .map_err(|_| anyhow::anyhow!("Decryption failed (Bad Key or Tampered Data)"))?;
            
        let entry: LedgerEntry = serde_json::from_slice(&plaintext)?;
        Ok(entry)
    }
}

/// 加密的操作载荷 (Envelope Body)
///
/// **结构**:
/// - `doc_id`: 明文，用于路由。
/// - `seq`: 明文，用于 Vector Clock 排序。
/// - `ciphertext`: 密文 (LedgerEntry)。
/// - `nonce`: 用于 AES-GCM 解密的随机数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedOp {
    pub doc_id: DocId,
    pub seq: u64,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Op;

    #[test]
    fn test_encrypt_decrypt() {
        let key = RepoKey::generate();
        let entry = LedgerEntry {
            doc_id: DocId::new(),
            op: Op::Insert { pos: 0, content: "Secret".to_string() },
            timestamp: 12345,
        };

        // Encrypt
        let enc = key.encrypt(&entry).unwrap();
        assert!(!enc.ciphertext.is_empty());
        assert_ne!(enc.ciphertext, serde_json::to_vec(&entry).unwrap()); // Ciphertext != Plaintext

        // Decrypt
        let dec = key.decrypt(&enc).unwrap();
        assert_eq!(dec.doc_id, entry.doc_id);
    }
}
