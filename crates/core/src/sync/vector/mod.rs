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

use crate::models::PeerId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::ops::Range;

/// 栈分配阈值：6 个协作者 (覆盖 99% 场景)
const INLINE_CAP: usize = 6;

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

    /// 合并另一个版本向量。取两者的最大值 (Union / Max)。
    ///
    /// **数学定义**: `result[p] = max(self[p], other[p])` for all p
    /// **复杂度**: O(n + m) 归并
    pub fn merge(&mut self, other: &VersionVector) {
        // 归并两个有序数组
        let mut result = SmallVec::<[(PeerId, u64); INLINE_CAP]>::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.clock.len() && j < other.clock.len() {
            let (ref p1, v1) = self.clock[i];
            let (ref p2, v2) = other.clock[j];

            match p1.cmp(p2) {
                std::cmp::Ordering::Less => {
                    result.push((p1.clone(), v1));
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    result.push((p2.clone(), v2));
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    result.push((p1.clone(), std::cmp::max(v1, v2)));
                    i += 1;
                    j += 1;
                }
            }
        }

        // 处理剩余元素
        while i < self.clock.len() {
            result.push(self.clock[i].clone());
            i += 1;
        }
        while j < other.clock.len() {
            result.push(other.clock[j].clone());
            j += 1;
        }

        self.clock = result;
    }
}

// 将 iter、intersection、diff 拆分到单独的 impl 块以保持文件简洁
impl VersionVector {
    /// 获取内部时钟的迭代器
    pub fn iter(&self) -> impl Iterator<Item = (&PeerId, &u64)> {
        self.clock.iter().map(|(p, v)| (p, v))
    }

    /// 计算两个向量的交集 (LCA - Lowest Common Ancestor)。
    ///
    /// **数学定义**: `result[p] = min(self[p], other[p])` for all p
    pub fn intersection(&self, other: &VersionVector) -> VersionVector {
        let mut result = VersionVector::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.clock.len() && j < other.clock.len() {
            let (ref p1, v1) = self.clock[i];
            let (ref p2, v2) = other.clock[j];

            match p1.cmp(p2) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    let min_v = std::cmp::min(v1, v2);
                    if min_v > 0 {
                        result.clock.push((p1.clone(), min_v));
                    }
                    i += 1;
                    j += 1;
                }
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
    pub fn diff(&self, remote: &VersionVector) -> VvDiffResult {
        let mut missing_from_remote = Vec::new();
        let mut missing_from_local = Vec::new();

        let mut i = 0;
        let mut j = 0;

        while i < self.clock.len() && j < remote.clock.len() {
            let (ref p1, v1) = self.clock[i];
            let (ref p2, v2) = remote.clock[j];

            match p1.cmp(p2) {
                std::cmp::Ordering::Less => {
                    // 我有，对方没有
                    missing_from_remote.push((p1.clone(), 1..(v1 + 1)));
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    // 对方有，我没有
                    missing_from_local.push((p2.clone(), 1..(v2 + 1)));
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    if v1 > v2 {
                        missing_from_remote.push((p1.clone(), (v2 + 1)..(v1 + 1)));
                    } else if v2 > v1 {
                        missing_from_local.push((p1.clone(), (v1 + 1)..(v2 + 1)));
                    }
                    i += 1;
                    j += 1;
                }
            }
        }

        // 处理剩余
        while i < self.clock.len() {
            let (ref p, v) = self.clock[i];
            missing_from_remote.push((p.clone(), 1..(v + 1)));
            i += 1;
        }
        while j < remote.clock.len() {
            let (ref p, v) = remote.clock[j];
            missing_from_local.push((p.clone(), 1..(v + 1)));
            j += 1;
        }

        (missing_from_remote, missing_from_local)
    }
}
