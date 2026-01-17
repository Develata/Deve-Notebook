// crates\core\src\security
//! # 哈希计算 (Hashing)
//!
//! **功能**:
//! 提供统一的 Hash 算法 (SHA256) 接口，用于从公钥派生 PeerID，或计算内容指纹。
//!
//! **设计**:
//! - 使用 SHA2-256。
//! - 输出十六进制字符串或原始字节。

use sha2::{Sha256, Digest};

/// 计算数据的 SHA256 哈希，返回十六进制字符串
/// 
/// **用途**:
/// - PeerID 生成: `hash(pub_key)`
/// - 数据完整性校验
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// 计算数据的 SHA256 哈希，返回字节数组
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let input = b"hello world";
        let hex = sha256_hex(input);
        assert_eq!(hex, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }
}
