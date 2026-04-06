use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SWITCH_NONCE: AtomicU64 = AtomicU64::new(1);

pub fn next_switch_nonce_after(current_scope_nonce: u64) -> u64 {
    let candidate = NEXT_SWITCH_NONCE.fetch_add(1, Ordering::Relaxed);
    if candidate > current_scope_nonce {
        return candidate;
    }

    let next = current_scope_nonce + 1;
    let _ = NEXT_SWITCH_NONCE.fetch_max(next + 1, Ordering::Relaxed);
    next
}

#[cfg(test)]
mod tests {
    use super::next_switch_nonce_after;

    #[test]
    fn switch_nonce_is_always_greater_than_current_scope() {
        let nonce = next_switch_nonce_after(1);
        assert!(nonce > 1);
    }

    #[test]
    fn switch_nonce_never_regresses_after_a_large_scope_nonce() {
        let first = next_switch_nonce_after(41);
        let second = next_switch_nonce_after(1);
        assert!(first > 41);
        assert!(second >= first);
    }
}
