//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 10_rendering#document-authority-bridge
//!
use std::sync::{Arc, Mutex};

/// 清空编辑器侧的内存缓冲；锁损坏时仅记录并 fail-closed。
pub fn clear_locked_vec<T>(buffer: &Arc<Mutex<Vec<T>>>, label: &str) {
    match buffer.lock() {
        Ok(mut buffered) => buffered.clear(),
        Err(_) => leptos::logging::warn!("忽略 {}: 锁已损坏", label),
    }
}

pub fn clear_sync_buffers<T, U>(
    live_ops: &Arc<Mutex<Vec<T>>>,
    encrypted_ops: &Arc<Mutex<Vec<U>>>,
    live_label: &str,
    encrypted_label: &str,
) {
    clear_locked_vec(live_ops, live_label);
    clear_locked_vec(encrypted_ops, encrypted_label);
}
