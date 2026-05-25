//! UI 偏好存储层。
//! plan_ref:
//!   - 11_ui_design_01_web#web-layout-persistence
//!   - 15_settings#browser-ui-prefs
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

/// 读取整数 UI 偏好。
pub fn read_i32_pref(key: &str) -> Option<i32> {
    read_pref(key)?.parse::<i32>().ok()
}

/// 写入整数 UI 偏好。
pub fn write_i32_pref(key: &str, value: i32) -> StorageResult<()> {
    write_pref(key, &value.to_string())
}

/// 读取布尔 UI 偏好。
pub fn read_bool_pref(key: &str) -> Option<bool> {
    match read_pref(key)?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// 写入布尔 UI 偏好。
pub fn write_bool_pref(key: &str, value: bool) -> StorageResult<()> {
    write_pref(key, if value { "true" } else { "false" })
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

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.set_item("__deve_prefs_probe__", "1").ok()?;
    let _ = storage.remove_item("__deve_prefs_probe__");
    Some(storage)
}

#[cfg(not(target_arch = "wasm32"))]
fn local_storage() -> Option<web_sys::Storage> {
    None
}

fn memory_get(key: &str) -> Option<String> {
    MEMORY_PREFS.with(|prefs| prefs.borrow().get(key).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &str = "__deve_test_pref__";

    #[test]
    fn typed_prefs_roundtrip_through_fallback_layer() {
        remove_pref(TEST_KEY);

        write_i32_pref(TEST_KEY, 320).expect("write i32 pref");
        assert_eq!(read_i32_pref(TEST_KEY), Some(320));

        write_bool_pref(TEST_KEY, true).expect("write bool pref");
        assert_eq!(read_bool_pref(TEST_KEY), Some(true));

        write_pref(TEST_KEY, "invalid").expect("write invalid bool");
        assert_eq!(read_bool_pref(TEST_KEY), None);

        remove_pref(TEST_KEY);
        assert_eq!(read_pref(TEST_KEY), None);
    }
}
