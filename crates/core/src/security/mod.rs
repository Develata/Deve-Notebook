// crates\core\src\security
//! # 安全模块 (Security Module)
//!
//! **架构作用**:
//! 提供网络层所需的安全原语，包括节点身份认证 (Ed25519) 和数据传输加密 (AES-GCM)。
//!
//! **核心组件**:
//! - `hashing`: Hash 计算 (SHA256) 用于 PeerID 生成。
//! - `keypair`: 身份密钥对 (Identity Key) 管理。
//! - `cipher`: 对称加密 (Repo Key) 逻辑。
//!
//! **类型**: Core MUST (核心必选)

pub mod hashing;
pub mod keypair;
pub mod cipher;

// Re-exports
pub use self::keypair::IdentityKeyPair;
pub use self::cipher::{RepoKey, EncryptedOp};
