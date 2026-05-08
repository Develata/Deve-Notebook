//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
use std::sync::atomic::{AtomicU64, Ordering};

use deve_core::protocol::{ScopeNonce, SwitchNonce};

static NEXT_SWITCH_NONCE: AtomicU64 = AtomicU64::new(1);

pub fn next_switch_nonce_after(current_scope_nonce: u64) -> Option<u64> {
    let current_scope_nonce = ScopeNonce::new(current_scope_nonce);
    let candidate = NEXT_SWITCH_NONCE.fetch_add(1, Ordering::Relaxed);
    let candidate = SwitchNonce::new(candidate);
    if candidate.is_after_scope(current_scope_nonce) {
        return Some(candidate.get());
    }

    let next = current_scope_nonce.next_switch_nonce()?;
    let _ = NEXT_SWITCH_NONCE.fetch_max(next.get().saturating_add(1), Ordering::Relaxed);
    Some(next.get())
}

#[cfg(test)]
mod tests {
    use super::next_switch_nonce_after;

    #[test]
    fn switch_nonce_is_always_greater_than_current_scope() {
        let nonce = next_switch_nonce_after(1).expect("switch nonce");
        assert!(nonce > 1);
    }

    #[test]
    fn switch_nonce_never_regresses_after_a_large_scope_nonce() {
        let first = next_switch_nonce_after(41).expect("first switch nonce");
        let second = next_switch_nonce_after(1).expect("second switch nonce");
        assert!(first > 41);
        assert!(second >= first);
    }

    #[test]
    fn max_scope_nonce_has_no_valid_switch_nonce() {
        assert_eq!(next_switch_nonce_after(u64::MAX), None);
    }
}
