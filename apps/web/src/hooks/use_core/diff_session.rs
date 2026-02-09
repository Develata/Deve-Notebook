//! Diff 会话线协议对象。
//!
//! Invariants:
//! - `path` 必须是非空规范化路径。
//! - `old_content` 与 `new_content` 必须来源于同一文件快照对。
//! - `opened_at_ms` 单调表示最近一次打开 Diff 的时间戳。

#[derive(Clone, Debug, PartialEq)]
pub struct DiffSessionWire {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
    pub opened_at_ms: u64,
}

impl DiffSessionWire {
    pub fn new(path: String, old_content: String, new_content: String) -> Self {
        Self {
            path,
            old_content,
            new_content,
            opened_at_ms: js_sys::Date::now() as u64,
        }
    }
}
