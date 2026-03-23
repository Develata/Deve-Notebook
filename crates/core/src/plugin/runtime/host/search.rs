// crates/core/src/plugin/runtime/host/search.rs
//! # 文件搜索宿主函数
//!
//! **功能**: 提供 glob 搜索和正则 grep 能力给 Rhai 插件。
//! **安全**: 搜索范围限定在项目根目录内，遵守 .gitignore。
//!
//! ## Invariants
//! 1. 搜索结果最多返回 MAX_RESULTS 条，防止内存溢出
//! 2. glob 搜索遵守 .gitignore 规则（通过 ignore crate）
//! 3. grep 仅搜索文本文件（跳过二进制文件）

use crate::plugin::manifest::Capability;
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use regex::Regex;
use rhai::{Engine, EvalAltResult};
use std::sync::Arc;

#[path = "search_output.rs"]
mod output;
#[path = "search_scope.rs"]
mod scope;

/// 最大返回结果数（768 MB 内存安全阈值）
const MAX_RESULTS: usize = 200;

/// 注册搜索相关 API
pub fn register_search_api(engine: &mut Engine, caps: Arc<Capability>) {
    register_search_files(engine, caps.clone());
    register_grep_files(engine, caps);
}

/// API: search_files(pattern: &str) -> String
/// 使用 ignore crate 的 OverrideBuilder 进行 glob 匹配
fn register_search_files(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_search = caps.clone();
    engine.register_fn(
        "search_files",
        move |pattern: &str| -> Result<String, Box<EvalAltResult>> {
            if !caps_search.check_search() {
                return Err("Permission denied: search access not allowed by manifest.".into());
            }
            let root = std::env::current_dir().map_err(|e| e.to_string())?;

            let mut ovr = OverrideBuilder::new(&root);
            ovr.add(pattern).map_err(|e| format!("Invalid glob: {e}"))?;
            let ovr = ovr.build().map_err(|e| format!("Glob build: {e}"))?;

            let walker = WalkBuilder::new(&root)
                .hidden(true)
                .git_ignore(true)
                .max_depth(Some(8))
                .overrides(ovr)
                .build();

            let mut matches = Vec::new();
            for entry in walker.flatten() {
                if matches.len() >= MAX_RESULTS {
                    break;
                }
                if !entry.path().is_file() {
                    continue;
                }
                let rel = scope::relative_search_path(&root, entry.path())?;
                matches.push(rel);
            }

            output::format_file_results(pattern, &matches, MAX_RESULTS)
        },
    );
}

/// API: grep_files(pattern: &str, path: &str) -> String
fn register_grep_files(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_grep = caps.clone();
    engine.register_fn(
        "grep_files",
        move |pattern: &str, path: &str| -> Result<String, Box<EvalAltResult>> {
            if !caps_grep.check_search() {
                return Err("Permission denied: search access not allowed by manifest.".into());
            }
            let root = std::env::current_dir().map_err(|e| e.to_string())?;
            let search_root = scope::resolve_search_root(&root, path)?;
            let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {e}"))?;

            let walker = WalkBuilder::new(&search_root)
                .hidden(true)
                .git_ignore(true)
                .max_depth(Some(8))
                .build();

            let mut results = Vec::new();
            for entry in walker.flatten() {
                if results.len() >= MAX_RESULTS {
                    break;
                }
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(p) else {
                    continue;
                };
                output::collect_grep_matches(&re, p, &root, &content, &mut results, MAX_RESULTS)?;
            }

            output::format_grep_results(pattern, &results, MAX_RESULTS)
        },
    );
}
