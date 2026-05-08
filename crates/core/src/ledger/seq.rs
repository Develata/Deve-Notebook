//! plan_ref:
//!   - 04_storage#facts-partition

use serde::{Deserialize, Serialize};
use std::fmt;

/// Ledger-wide durable append order.
///
/// The on-disk redb key remains `u64`; this type marks the authority boundary
/// where the next ledger-wide sequence is allocated.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct GlobalSeq(u64);

impl GlobalSeq {
    pub const ZERO: Self = Self(0);

    pub const fn from_storage_key(value: u64) -> Self {
        Self(value)
    }

    pub const fn storage_key(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl From<GlobalSeq> for u64 {
    fn from(seq: GlobalSeq) -> Self {
        seq.storage_key()
    }
}

impl From<u64> for GlobalSeq {
    fn from(value: u64) -> Self {
        Self::from_storage_key(value)
    }
}

impl fmt::Display for GlobalSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) const fn checked_next_local_seq(current: u64) -> Option<u64> {
    current.checked_add(1)
}

#[cfg(test)]
mod tests {
    use super::{GlobalSeq, checked_next_local_seq};

    #[test]
    fn global_seq_zero_is_empty_ledger_anchor() {
        assert_eq!(GlobalSeq::ZERO.storage_key(), 0);
    }

    #[test]
    fn global_seq_next_allocates_next_storage_key() {
        let seq = GlobalSeq::from_storage_key(41)
            .next()
            .expect("next sequence must fit");
        assert_eq!(seq.storage_key(), 42);
    }

    #[test]
    fn global_seq_next_rejects_overflow() {
        assert!(GlobalSeq::from_storage_key(u64::MAX).next().is_none());
    }

    #[test]
    fn local_seq_next_rejects_overflow() {
        assert_eq!(checked_next_local_seq(41), Some(42));
        assert_eq!(checked_next_local_seq(u64::MAX), None);
    }
}
