//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 07_network#server-ws-runtime

use serde::{Deserialize, Deserializer, Serialize};
use smol_str::SmolStr;
use std::fmt;

/// Repo-scoped sequence allocated by one physical peer across every fact kind.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PeerFactSeq(u64);

impl PeerFactSeq {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl From<u64> for PeerFactSeq {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<PeerFactSeq> for u64 {
    fn from(value: PeerFactSeq) -> Self {
        value.get()
    }
}

impl PartialEq<u64> for PeerFactSeq {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<PeerFactSeq> for u64 {
    fn eq(&self, other: &PeerFactSeq) -> bool {
        *self == other.0
    }
}

impl fmt::Display for PeerFactSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Diagnostic producer label. It never participates in identity or ordering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct FactActor(SmolStr);

impl<'de> Deserialize<'de> for FactActor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl FactActor {
    pub const MAX_LEN: usize = 64;

    pub fn new(value: impl AsRef<str>) -> Result<Self, FactActorError> {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(FactActorError::Empty);
        }
        if value.len() > Self::MAX_LEN {
            return Err(FactActorError::TooLong {
                actual: value.len(),
            });
        }
        Ok(Self(SmolStr::new(value)))
    }

    pub fn system() -> Self {
        Self(SmolStr::new_static("system"))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for FactActor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FactActorError {
    #[error("fact actor must not be empty")]
    Empty,
    #[error("fact actor exceeds {max} bytes: {actual}", max = FactActor::MAX_LEN)]
    TooLong { actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_fact_seq_next_is_checked() {
        assert_eq!(PeerFactSeq::ZERO.next(), Some(PeerFactSeq::ONE));
        assert_eq!(PeerFactSeq::new(u64::MAX).next(), None);
    }

    #[test]
    fn fact_actor_rejects_empty_and_oversized_labels() {
        assert_eq!(FactActor::new(""), Err(FactActorError::Empty));
        assert_eq!(
            FactActor::new("x".repeat(FactActor::MAX_LEN + 1)),
            Err(FactActorError::TooLong {
                actual: FactActor::MAX_LEN + 1,
            })
        );
    }

    #[test]
    fn fact_actor_deserialization_preserves_constructor_invariants() {
        assert!(serde_json::from_str::<FactActor>(r#"""#).is_err());
        let oversized = serde_json::to_string(&"x".repeat(FactActor::MAX_LEN + 1)).unwrap();
        assert!(serde_json::from_str::<FactActor>(&oversized).is_err());
        assert_eq!(
            serde_json::from_str::<FactActor>(r#""merge""#)
                .unwrap()
                .as_str(),
            "merge"
        );
    }
}
