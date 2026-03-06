//! UI 偏好存储层。
//!
//! 该层只服务主题、布局、语言等无害偏好；当 `localStorage` 不可用时自动回退到内存态。

#![allow(dead_code)]

use super::{StorageError, StorageResult};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static MEMORY_PREFS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// 读取 UI 偏好键值。
pub fn read_pref(key: &str) -> Option<String> {
    local_storage()
        .and_then(|s| s.get_item(key).ok().flatten())
        .or_else(|| memory_get(key))
}

/// 写入 UI 偏好键值。
pub fn write_pref(key: &str, value: &str) -> StorageResult<()> {
    if let Some(storage) = local_storage() {
        return storage
            .set_item(key, value)
            .map_err(|e| StorageError::Browser(format!("{e:?}")));
    }
    MEMORY_PREFS.with(|prefs| {
        prefs.borrow_mut().insert(key.into(), value.into());
    });
    Ok(())
}

/// 删除 UI 偏好键值。
pub fn remove_pref(key: &str) {
    if let Some(storage) = local_storage() {
        let _ = storage.remove_item(key);
    }
    MEMORY_PREFS.with(|prefs| {
        prefs.borrow_mut().remove(key);
    });
}

fn local_storage() -> Option<web_sys::Storage> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.set_item("__deve_prefs_probe__", "1").ok()?;
    let _ = storage.remove_item("__deve_prefs_probe__");
    Some(storage)
}

fn memory_get(key: &str) -> Option<String> {
    MEMORY_PREFS.with(|prefs| prefs.borrow().get(key).cloned())
}
