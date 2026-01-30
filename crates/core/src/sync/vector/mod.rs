// crates\core\src\sync\vector
//! # 版本向量模块 (Version Vector)
//!
//! **架构作用**:
//! 定义用于追踪 P2P 数据一致性状态的逻辑时钟向量。
//! 它是 P2P 同步的"单一真理"，用于检测数据差异和并发冲突。
//!
//! **核心功能清单**:
//! - `VersionVector`: 封装 `HashMap<PeerId, u64>`，提供逻辑时钟管理。
//! - `diff`: 比较两个向量，计算出缺失的数据范围。
//!
//! **类型**: Core MUST (核心必选)
//!
//! ## 数学不变量 (Mathematical Invariants)
//!
//! 1. **单调性**: 对于任意 PeerId p，`self.get(p)` 只能单调递增。
//! 2. **幂等合并**: `v.merge(&v) == v` (合并自身不改变状态)。
//! 3. **交换律**: `v1.merge(&v2)` 与 `v2.merge(&v1)` 结果相同。

#[cfg(test)]
mod tests;

use crate::models::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

/// 版本向量差异结果类型
///
/// - 第一个元素：对方缺少的 (我比对方新的部分)
/// - 第二个元素：我缺少的 (对方比我新的部分)
pub type VvDiffResult = (Vec<(PeerId, Range<u64>)>, Vec<(PeerId, Range<u64>)>);

/// 逻辑时钟向量，用于追踪各个节点的数据同步状态。
///
/// ## 不变量 (Invariants)
///
/// - 所有存储的序列号均为正整数 (> 0)。
/// - 未记录的 PeerId 隐式视为 seq = 0。
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
    #[inline]
    pub fn get(&self, peer: &PeerId) -> u64 {
        *self.clock.get(peer).unwrap_or(&0)
    }

    /// 更新指定节点的版本号。
    ///
    /// **前置条件**: 无
    /// **后置条件**: `self.get(peer) >= seq` (单调性保证)
    pub fn update(&mut self, peer: PeerId, seq: u64) {
        let entry = self.clock.entry(peer).or_insert(0);
        if seq > *entry {
            *entry = seq;
        }
    }

    /// 合并另一个版本向量。取两者的最大值 (Union / Max)。
    ///
    /// **数学定义**: `result[p] = max(self[p], other[p])` for all p
    pub fn merge(&mut self, other: &VersionVector) {
        for (peer, &seq) in &other.clock {
            let entry = self.clock.entry(peer.clone()).or_insert(0);
            if seq > *entry {
                *entry = seq;
            }
        }
    }

    /// 获取内部时钟的迭代器
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, PeerId, u64> {
        self.clock.iter()
    }

    /// 计算两个向量的交集 (LCA - Lowest Common Ancestor)。
    ///
    /// **数学定义**: `result[p] = min(self[p], other[p])` for all p
    pub fn intersection(&self, other: &VersionVector) -> VersionVector {
        let mut result = VersionVector::new();
        let all_peers: Vec<&PeerId> = self.clock.keys().chain(other.clock.keys()).collect();

        for peer in all_peers {
            let v1 = self.get(peer);
            let v2 = other.get(peer);
            let min_v = std::cmp::min(v1, v2);
            if min_v > 0 {
                result.update(peer.clone(), min_v);
            }
        }
        result
    }

    /// 计算差异 (Diff)。
    ///
    /// 比较 "Self" (My State) 和 "Remote" (Their State)。
    ///
    /// ## 返回值
    ///
    /// 1. `missing_from_remote`: 对方缺少的 (我比对方新的部分)
    /// 2. `missing_from_local`: 我缺少的 (对方比我新的部分)
    ///
    /// ## Range 语义
    ///
    /// 使用 Rust 标准 `Range<u64>` (半开区间 `[start, end)`)。
    /// 例如：若 local=10, remote=5，则 missing_from_remote = `6..11`。
    pub fn diff(&self, remote: &VersionVector) -> VvDiffResult {
        let mut missing_from_remote = Vec::new();
        let mut missing_from_local = Vec::new();

        // 收集所有涉及的 PeerId 并去重
        let mut all_peers: Vec<&PeerId> = self.clock.keys().chain(remote.clock.keys()).collect();
        all_peers.sort();
        all_peers.dedup();

        for peer in all_peers {
            let my_ver = self.get(peer);
            let their_ver = remote.get(peer);

            if my_ver > their_ver {
                // 我有更多，对方缺 (their_ver+1 .. my_ver+1)
                missing_from_remote.push((peer.clone(), (their_ver + 1)..(my_ver + 1)));
            } else if their_ver > my_ver {
                // 对方有更多，我缺 (my_ver+1 .. their_ver+1)
                missing_from_local.push((peer.clone(), (my_ver + 1)..(their_ver + 1)));
            }
        }

        (missing_from_remote, missing_from_local)
    }
}
