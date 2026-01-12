//! # 版本向量模块 (Version Vector)
//!
//! **架构作用**:
//! 定义用于追踪 P2P 数据一致性状态的逻辑时钟向量。它是 P2P 同步的“单一真理”，用于检测数据差异和并发冲突。
//!
//! **核心功能清单**:
//! - `VersionVector`: 封装 `HashMap<PeerId, u64>`，提供逻辑时钟管理。
//! - `diff`: 比较两个向量，计算出缺失的数据范围。
//!
//! **类型**: Core MUST (核心必选)

use std::collections::HashMap;
use std::ops::Range;
use serde::{Deserialize, Serialize};
use crate::models::PeerId;

/// 逻辑时钟向量，用于追踪各个节点的数据同步状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionVector {
    /// 映射: 节点ID -> 该节点已知的最大操作序列号 (Sequence Number)
    clock: HashMap<PeerId, u64>,
}

impl VersionVector {
    /// 创建一个新的空版本向量
    pub fn new() -> Self {
        Self {
            clock: HashMap::new(),
        }
    }

    /// 获取指定节点的当前版本号。如果不存在，返回 0。
    pub fn get(&self, peer: &PeerId) -> u64 {
        *self.clock.get(peer).unwrap_or(&0)
    }

    /// 更新指定节点的版本号。只有当新版本号更大时才更新。
    pub fn update(&mut self, peer: PeerId, seq: u64) {
        let entry = self.clock.entry(peer).or_insert(0);
        if seq > *entry {
            *entry = seq;
        }
    }

    /// 合并另一个版本向量。取两者的最大值 (Union / Max)。
    pub fn merge(&mut self, other: &VersionVector) {
        for (peer, &seq) in &other.clock {
            let entry = self.clock.entry(peer.clone()).or_insert(0);
            if seq > *entry {
                *entry = seq;
            }
        }
    }

    /// 计算差异。
    /// 比较 "Self" (My State) 和 "Remote" (Their State)。
    /// 返回两个缺失范围列表：
    /// 1. `missing_from_remote`: 对方缺少的 (我比对方新的部分)
    /// 2. `missing_from_local`: 我缺少的 (对方比我新的部分)
    pub fn diff(&self, remote: &VersionVector) -> (Vec<(PeerId, Range<u64>)>, Vec<(PeerId, Range<u64>)>) {
        let mut missing_from_remote = Vec::new();
        let mut missing_from_local = Vec::new();

        // 收集所有涉及的 PeerId
        let mut all_peers: Vec<&PeerId> = self.clock.keys().chain(remote.clock.keys()).collect();
        all_peers.sort();
        all_peers.dedup();

        for peer in all_peers {
            let my_ver = self.get(peer);
            let their_ver = remote.get(peer);

            if my_ver > their_ver {
                // 我有更多，对方缺 (their_ver..my_ver]
                // Range excludes end, but conceptually it is items from their_ver+1 up to my_ver.
                // In Rust Range: start..end includes start, excludes end.
                // Wait, if I have 10 and they have 5. They need 6, 7, 8, 9, 10.
                // Range should be (their_ver + 1)..=my_ver?
                // Ledger usually uses `seq`. If they have 5, they have processed seq 5. The next is 6.
                // So range is (their_ver + 1)..(my_ver + 1) in Rust range notation?
                // NOTE: The prompt says `Range<u64>`.
                // Let's assume the user means semi-open range relevant to standard Rust `Range`.
                // If they have 5, they miss 6..=10.
                // Rust range `6..11`.
                missing_from_remote.push((peer.clone(), (their_ver + 1)..(my_ver + 1)));
            } else if their_ver > my_ver {
                // 对方有更多，我缺 (my_ver..their_ver]
                missing_from_local.push((peer.clone(), (my_ver + 1)..(their_ver + 1)));
            }
        }

        (missing_from_remote, missing_from_local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str) -> PeerId {
        PeerId::new(id)
    }

    #[test]
    fn test_vector_update_merge() {
        let mut v1 = VersionVector::new();
        v1.update(p("A"), 10);
        v1.update(p("B"), 5);

        assert_eq!(v1.get(&p("A")), 10);
        assert_eq!(v1.get(&p("B")), 5);
        assert_eq!(v1.get(&p("C")), 0);

        let mut v2 = VersionVector::new();
        v2.update(p("B"), 10);
        v2.update(p("C"), 7);

        v1.merge(&v2);

        assert_eq!(v1.get(&p("A")), 10);
        assert_eq!(v1.get(&p("B")), 10); // Updated to max
        assert_eq!(v1.get(&p("C")), 7);  // New entry
    }

    #[test]
    fn test_diff_scenarios() {
        let mut local = VersionVector::new();
        let mut remote = VersionVector::new();

        // Scenario 1: A leading (Local has more data from A)
        local.update(p("A"), 10);
        remote.update(p("A"), 5);
        
        let (missing_remote, missing_local) = local.diff(&remote);
        assert!(!missing_remote.is_empty());
        assert!(missing_local.is_empty());
        let (id, range) = &missing_remote[0];
        assert_eq!(id, &p("A"));
        assert_eq!(range, &(6..11)); // Expecting 6,7,8,9,10.

        // Scenario 2: B leading (Remote has more data from B)
        local.update(p("B"), 3);
        remote.update(p("B"), 8);

        let (missing_remote, missing_local) = local.diff(&remote);
        // From previous step (A): Local A=10, Remote A=5. -> Missing Remote: A 6..11
        // From this step (B): Local B=3, Remote B=8. -> Missing Local: B 4..9
        
        // Check missing from remote (A case)
        assert!(missing_remote.iter().any(|(id, r)| id == &p("A") && r == &(6..11)));
        
        // Check missing from local (B case)
        assert!(missing_local.iter().any(|(id, r)| id == &p("B") && r == &(4..9)));
    }

    #[test]
    fn test_diff_concurrent() {
        // 分叉/并发场景
        // Local: { A: 10, B: 20 }
        // Remote: { A: 12, B: 15 }
        let mut local = VersionVector::new();
        local.update(p("A"), 10);
        local.update(p("B"), 20);

        let mut remote = VersionVector::new();
        remote.update(p("A"), 12);
        remote.update(p("B"), 15);

        let (missing_remote, missing_local) = local.diff(&remote);

        // Remote needs B: 16..21
        assert!(missing_remote.iter().any(|(id, r)| id == &p("B") && r == &(16..21)));
        
        // Local needs A: 11..13
        assert!(missing_local.iter().any(|(id, r)| id == &p("A") && r == &(11..13)));
    }
}
