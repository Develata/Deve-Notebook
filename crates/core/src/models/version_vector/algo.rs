//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::{INLINE_CAP, VersionVector, VvDiffResult};
use super::{PeerFactSeq, PeerId};
use smallvec::SmallVec;
impl VersionVector {
    /// 合并另一个版本向量。取两者的最大值 (Union / Max)。
    /// **数学定义**: `result[p] = max(self[p], other[p])` for all p
    /// **复杂度**: O(n + m) 归并
    pub fn merge(&mut self, other: &VersionVector) {
        // 归并两个有序数组
        let mut result = SmallVec::<[(PeerId, PeerFactSeq); INLINE_CAP]>::new();
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

    /// 计算差异 (Diff)。
    /// 比较 "Self" (My State) 和 "Remote" (Their State)。
    /// 返回: `missing_from_remote` 与 `missing_from_local`。
    /// 返回闭区间 `[start, end]`，与 wire range 一致。
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
                    missing_from_remote.push((p1.clone(), (PeerFactSeq::ONE, v1)));
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    // 对方有，我没有
                    missing_from_local.push((p2.clone(), (PeerFactSeq::ONE, v2)));
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    if v1 > v2 {
                        missing_from_remote.push((
                            p1.clone(),
                            (v2.next().expect("larger peer seq has a successor"), v1),
                        ));
                    } else if v2 > v1 {
                        missing_from_local.push((
                            p1.clone(),
                            (v1.next().expect("larger peer seq has a successor"), v2),
                        ));
                    }
                    i += 1;
                    j += 1;
                }
            }
        }

        // 处理剩余
        while i < self.clock.len() {
            let (ref p, v) = self.clock[i];
            missing_from_remote.push((p.clone(), (PeerFactSeq::ONE, v)));
            i += 1;
        }
        while j < remote.clock.len() {
            let (ref p, v) = remote.clock[j];
            missing_from_local.push((p.clone(), (PeerFactSeq::ONE, v)));
            j += 1;
        }

        (missing_from_remote, missing_from_local)
    }
}
