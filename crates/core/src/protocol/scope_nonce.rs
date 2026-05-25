//! Repo scope nonce protocol entities.
//! plan_ref:
//!   - 07_network#repo-scoped-handshake
//!   - 09_web_thin_client_ledger#write-readiness

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ScopeNonce(u64);

impl ScopeNonce {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn matches_optional(self, value: Option<u64>) -> bool {
        value == Some(self.0)
    }

    pub fn next_switch_nonce(self) -> Option<SwitchNonce> {
        self.0.checked_add(1).map(SwitchNonce::new)
    }
}

impl From<u64> for ScopeNonce {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<ScopeNonce> for u64 {
    fn from(value: ScopeNonce) -> Self {
        value.get()
    }
}

impl fmt::Display for ScopeNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct SwitchNonce(u64);

impl SwitchNonce {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn is_after_scope(self, scope: ScopeNonce) -> bool {
        self.0 > scope.get()
    }
}

impl From<u64> for SwitchNonce {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<SwitchNonce> for u64 {
    fn from(value: SwitchNonce) -> Self {
        value.get()
    }
}

impl fmt::Display for SwitchNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{ScopeNonce, SwitchNonce};

    #[test]
    fn scope_nonce_serializes_as_plain_number() {
        let encoded = serde_json::to_string(&ScopeNonce::new(17)).expect("encode");
        assert_eq!(encoded, "17");
        let decoded: ScopeNonce = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded.get(), 17);
    }

    #[test]
    fn switch_nonce_serializes_as_plain_number() {
        let encoded = serde_json::to_string(&SwitchNonce::new(19)).expect("encode");
        assert_eq!(encoded, "19");
        let decoded: SwitchNonce = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded.get(), 19);
    }

    #[test]
    fn scope_nonce_matches_optional_wire_value() {
        let nonce = ScopeNonce::new(23);
        assert!(nonce.matches_optional(Some(23)));
        assert!(!nonce.matches_optional(Some(22)));
        assert!(!nonce.matches_optional(None));
    }

    #[test]
    fn switch_nonce_must_be_strictly_after_scope_nonce() {
        let scope = ScopeNonce::new(41);
        assert!(SwitchNonce::new(42).is_after_scope(scope));
        assert!(!SwitchNonce::new(41).is_after_scope(scope));
    }

    #[test]
    fn max_scope_nonce_has_no_next_switch_nonce() {
        assert_eq!(ScopeNonce::new(u64::MAX).next_switch_nonce(), None);
    }
}
