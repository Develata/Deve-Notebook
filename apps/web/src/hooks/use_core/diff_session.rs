//! Diff 会话线协议对象。
//!
//! Invariants:
//! - `path` 必须是非空规范化路径。
//! - `display_path` 仅用于 UI 标题；默认等于 `path`，重命名视图可覆盖。
//! - `old_content` 与 `new_content` 必须来源于同一文件快照对。
//! - `opened_at_ms` 单调表示最近一次打开 Diff 的时间戳。

#[derive(Clone, Debug, PartialEq)]
pub struct DiffSessionWire {
    pub path: String,
    pub display_path: String,
    pub old_content: String,
    pub new_content: String,
    pub opened_at_ms: u64,
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> u64 {
    0
}

impl DiffSessionWire {
    pub fn new(path: String, old_content: String, new_content: String) -> Self {
        let display_path = path.clone();
        Self::with_display_path(path, display_path, old_content, new_content)
    }

    pub fn with_display_path(
        path: String,
        display_path: String,
        old_content: String,
        new_content: String,
    ) -> Self {
        Self {
            path,
            display_path,
            old_content,
            new_content,
            opened_at_ms: now_ms(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DiffSessionWire;

    #[test]
    fn defaults_display_path_to_canonical_path() {
        let session = DiffSessionWire::new("notes/new.md".into(), "old".into(), "new".into());
        assert_eq!(session.path, "notes/new.md");
        assert_eq!(session.display_path, "notes/new.md");
    }

    #[test]
    fn keeps_display_label_separate_from_canonical_path() {
        let session = DiffSessionWire::with_display_path(
            "notes/new.md".into(),
            "notes/old.md -> notes/new.md".into(),
            "old".into(),
            "new".into(),
        );
        assert_eq!(session.path, "notes/new.md");
        assert_eq!(session.display_path, "notes/old.md -> notes/new.md");
    }
}
