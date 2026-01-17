// crates\core\src\utils
//! # 稳定哈希工具 (Stable Hash Utils)
//!
//! 提供稳定的哈希实现，用于跨进程/重启保持一致的 ID 生成。
//! 主要用于将文件系统 ID 映射到稳定的 u128 以供 Redb 使用。

use std::hash::Hasher;

/// FNV-1a 64位哈希算法的稳健实现。
/// 用于从 FileId 生成一致的 Inode ID。
pub struct StableHasher {
    state: u64,
}

impl StableHasher {
    pub fn new() -> Self {
        Self {
            state: 0xcbf29ce484222325,
        }
    }
}

impl Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

impl Default for StableHasher {
    fn default() -> Self {
        Self::new()
    }
}
