// crates/core/src/plugin/runtime/host/git.rs
//! # 版本控制宿主函数
//!
//! **功能**: 提供 Git-like 源代码控制能力。
//! **安全**: 需通过 Capability 的 source_control 检查。

use crate::plugin::manifest::Capability;
use rhai::{Engine, EvalAltResult};
use std::sync::Arc;

/// 截断文本，防止输出过长
fn truncate_text(input: &str, max_lines: usize, max_line_chars: usize) -> String {
    let mut output = String::new();
    let mut line_count = 0usize;

    for line in input.lines() {
        if line_count >= max_lines {
            output.push_str("... [Truncated]\n");
            break;
        }
        let mut truncated_line = line.to_string();
        if truncated_line.chars().count() > max_line_chars {
            truncated_line = truncated_line.chars().take(max_line_chars).collect();
            truncated_line.push_str("...");
        }
        output.push_str(&truncated_line);
        output.push('\n');
        line_count += 1;
    }

    output
}

/// 注册版本控制 API
pub fn register_git_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_status = caps.clone();
    let caps_diff = caps.clone();
    let caps_stage = caps.clone();
    let caps_commit = caps.clone();

    // API: sc_status() -> Dynamic (变更列表)
    engine.register_fn(
        "sc_status",
        move || -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_status.check_source_control() {
                return Err("Permission denied: source control access not allowed.".into());
            }
            let repo = super::repository().map_err(|e| e.to_string())?;
            let mut changes = repo.list_changes().map_err(|e| e.to_string())?;
            let max_changes = 50usize;
            if changes.len() > max_changes {
                let total = changes.len();
                changes.truncate(max_changes);
                changes.push(crate::source_control::ChangeEntry {
                    path: format!("... and {} more files", total - max_changes),
                    status: crate::source_control::ChangeStatus::Modified,
                });
            }
            let json = serde_json::to_value(&changes).map_err(|e| e.to_string())?;
            rhai::serde::to_dynamic(&json).map_err(|e| e.to_string().into())
        },
    );

    // API: sc_diff(path: &str) -> String
    engine.register_fn(
        "sc_diff",
        move |path: &str| -> Result<String, Box<EvalAltResult>> {
            if !caps_diff.check_source_control() {
                return Err("Permission denied: source control access not allowed.".into());
            }
            let repo = super::repository().map_err(|e| e.to_string())?;
            let diff = repo.diff_doc_path(path).map_err(|e| e.to_string())?;
            Ok(truncate_text(&diff, 200, 240))
        },
    );

    // API: sc_stage(path: &str)
    engine.register_fn(
        "sc_stage",
        move |path: &str| -> Result<(), Box<EvalAltResult>> {
            if !caps_stage.check_source_control() {
                return Err("Permission denied: source control access not allowed.".into());
            }
            let repo = super::repository().map_err(|e| e.to_string())?;
            repo.stage_file(path).map_err(|e| e.to_string().into())
        },
    );

    // API: sc_commit(message: &str) -> Dynamic
    engine.register_fn(
        "sc_commit",
        move |message: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_commit.check_source_control() {
                return Err("Permission denied: source control access not allowed.".into());
            }
            let repo = super::repository().map_err(|e| e.to_string())?;
            let commit = repo.commit_staged(message).map_err(|e| e.to_string())?;
            let json = serde_json::to_value(&commit).map_err(|e| e.to_string())?;
            rhai::serde::to_dynamic(&json).map_err(|e| e.to_string().into())
        },
    );
}
