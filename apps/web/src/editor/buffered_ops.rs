use std::sync::{Arc, Mutex};

/// 清空编辑器侧的内存缓冲；锁损坏时仅记录并 fail-closed。
pub fn clear_locked_vec<T>(buffer: &Arc<Mutex<Vec<T>>>, label: &str) {
    match buffer.lock() {
        Ok(mut buffered) => buffered.clear(),
        Err(_) => leptos::logging::warn!("忽略 {}: 锁已损坏", label),
    }
}
