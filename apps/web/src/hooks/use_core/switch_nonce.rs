use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SWITCH_NONCE: AtomicU64 = AtomicU64::new(1);

pub fn next_switch_nonce() -> u64 {
    NEXT_SWITCH_NONCE.fetch_add(1, Ordering::Relaxed)
}
