// crates\core\src\sync\vector
//! # 版本向量模块 (Version Vector)
//!
//! **架构作用**:
//! 定义用于追踪 P2P 数据一致性状态的逻辑时钟向量。
//! 它是 P2P 同步的"单一真理"，用于检测数据差异和并发冲突。
//!
//! ## 内存优化 (v2)
//!
//! 使用 `SmallVec<[(PeerId, u64); 6]>` 代替 `HashMap`：
//! - **零堆分配**: 协作者 ≤ 6 人时，整个向量在栈上。
//! - **缓存友好**: 连续内存布局，线性扫描比哈希查找更快（小规模）。
//! - **序列化高效**: 内存连续，序列化接近 memcpy。
//!
//! ## 数学不变量 (Mathematical Invariants)
//!
//! 1. **单调性**: 对于任意 PeerId p，`self.get(p)` 只能单调递增。
//! 2. **幂等合并**: `v.merge(&v) == v` (合并自身不改变状态)。
//! 3. **交换律**: `v1.merge(&v2)` 与 `v2.merge(&v1)` 结果相同。
//! 4. **有序性**: 内部数组按 PeerId 升序排列 (二分查找前提)。

#[cfg(test)]
mod tests;

mod algo;

use crate::models::PeerId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::ops::Range;

/// 栈分配阈值：6 个协作者 (覆盖 99% 场景)
pub(super) const INLINE_CAP: usize = 6;

/// 版本向量差异结果类型
///
/// - 第一个元素：对方缺少的 (我比对方新的部分)
/// - 第二个元素：我缺少的 (对方比我新的部分)
pub type VvDiffResult = (Vec<(PeerId, Range<u64>)>, Vec<(PeerId, Range<u64>)>);

/// 逻辑时钟向量，用于追踪各个节点的数据同步状态。
///
/// ## 内部结构
///
/// 使用有序数组 `SmallVec<[(PeerId, u64); 6]>` 存储，按 PeerId 升序排列。
/// 查找使用二分搜索 O(log n)，插入使用保序插入 O(n)。
/// 对于典型的小规模协作 (< 10 人)，线性扫描实际比哈希更快。
///
/// ## 不变量 (Invariants)
///
/// - 所有存储的序列号均为正整数 (> 0)。
/// - 未记录的 PeerId 隐式视为 seq = 0。
/// - 数组按 PeerId 严格升序排列，无重复键。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionVector {
    /// 有序数组: (PeerId, seq) 按 PeerId 升序排列
    clock: SmallVec<[(PeerId, u64); INLINE_CAP]>,
}

impl VersionVector {
    /// 创建一个新的空版本向量
    #[inline]
    pub fn new() -> Self {
        Self {
            clock: SmallVec::new(),
        }
    }

    /// 获取指定节点的当前版本号。如果不存在，返回 0。
    ///
    /// **复杂度**: O(log n) 二分查找
    #[inline]
    pub fn get(&self, peer: &PeerId) -> u64 {
        match self.clock.binary_search_by(|(p, _)| p.cmp(peer)) {
            Ok(idx) => self.clock[idx].1,
            Err(_) => 0,
        }
    }

    /// 更新指定节点的版本号。
    ///
    /// **前置条件**: 无
    /// **后置条件**: `self.get(peer) >= seq` (单调性保证)
    /// **复杂度**: O(log n) 查找 + O(n) 插入 (最坏情况)
    pub fn update(&mut self, peer: PeerId, seq: u64) {
        match self.clock.binary_search_by(|(p, _)| p.cmp(&peer)) {
            Ok(idx) => {
                // 已存在，只在新值更大时更新 (单调性)
                if seq > self.clock[idx].1 {
                    self.clock[idx].1 = seq;
                }
            }
            Err(idx) => {
                // 不存在，保序插入
                if seq > 0 {
                    self.clock.insert(idx, (peer, seq));
                }
            }
        }
    }

    /// 对内部数组进行排序与去重 (保持最大 seq)
    pub fn normalize(&mut self) {
        self.clock.sort_by(|(a, _), (b, _)| a.cmp(b));
        self.clock
            .dedup_by(|(a_peer, a_seq), (b_peer, b_seq)| {
                if a_peer == b_peer {
                    if *b_seq > *a_seq {
                        *a_seq = *b_seq;
                    }
                    true
                } else {
                    false
                }
            });
    }
}

// 将 iter 拆分到单独的 impl 块以保持文件简洁
impl VersionVector {
    /// 获取内部时钟的迭代器
    pub fn iter(&self) -> impl Iterator<Item = (&PeerId, &u64)> {
        self.clock.iter().map(|(p, v)| (p, v))
    }
}
