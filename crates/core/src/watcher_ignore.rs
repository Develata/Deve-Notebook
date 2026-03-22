// crates/core/src/watcher_ignore.rs
//! # 用户自定义忽略规则
//!
//! 解析 vault 根下的 `.deveignore` 文件，格式兼容 `.gitignore`。
//! Watcher 在处理文件事件前检查路径是否匹配忽略规则。

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

/// 用户自定义忽略规则集合。
pub struct IgnoreRules {
    matcher: Gitignore,
}

impl IgnoreRules {
    /// 从 vault 根目录加载 `.deveignore` 文件。
    ///
    /// 文件不存在或无法解析时返回空规则集（不阻塞启动）。
    pub fn load(vault_root: &Path) -> Self {
        let ignore_path = vault_root.join(".deveignore");
        let mut builder = GitignoreBuilder::new(vault_root);
        if ignore_path.is_file()
            && let Some(err) = builder.add(&ignore_path)
        {
            tracing::warn!("Failed to parse .deveignore: {}", err);
        }
        let matcher = builder.build().unwrap_or_else(|err| {
            tracing::warn!("Failed to build ignore rules: {}", err);
            Gitignore::empty()
        });
        Self { matcher }
    }

    /// 检查相对路径是否被忽略。
    pub fn is_ignored(&self, rel_path: &str) -> bool {
        self.matcher
            .matched_path_or_any_parents(rel_path, false)
            .is_ignore()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn empty_vault_returns_no_rules() {
        let dir = tempdir().expect("tempdir");
        let rules = IgnoreRules::load(dir.path());
        assert!(!rules.is_ignored("notes/a.md"));
    }

    #[test]
    fn deveignore_filters_matching_patterns() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".deveignore"), "*.tmp\ndrafts/\n").expect("write");
        let rules = IgnoreRules::load(dir.path());
        assert!(rules.is_ignored("notes/scratch.tmp"));
        assert!(rules.is_ignored("drafts/wip.md"));
        assert!(!rules.is_ignored("notes/keep.md"));
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".deveignore"), "# comment\n\n*.log\n").expect("write");
        let rules = IgnoreRules::load(dir.path());
        assert!(rules.is_ignored("server.log"));
        assert!(!rules.is_ignored("notes/a.md"));
    }
}
