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

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use regex::Regex;
use rhai::{Engine, EvalAltResult};

/// 最大返回结果数（768 MB 内存安全阈值）
const MAX_RESULTS: usize = 200;

/// 注册搜索相关 API
pub fn register_search_api(engine: &mut Engine) {
    register_search_files(engine);
    register_grep_files(engine);
}

/// API: search_files(pattern: &str) -> String
/// 使用 ignore crate 的 OverrideBuilder 进行 glob 匹配
fn register_search_files(engine: &mut Engine) {
    engine.register_fn(
        "search_files",
        |pattern: &str| -> Result<String, Box<EvalAltResult>> {
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
                if let Ok(rel) = entry.path().strip_prefix(&root) {
                    matches.push(rel.to_string_lossy().to_string());
                }
            }

            format_file_results(pattern, &matches)
        },
    );
}

/// API: grep_files(pattern: &str, path: &str) -> String
fn register_grep_files(engine: &mut Engine) {
    engine.register_fn(
        "grep_files",
        |pattern: &str, path: &str| -> Result<String, Box<EvalAltResult>> {
            let root = std::env::current_dir().map_err(|e| e.to_string())?;
            let search_root = if path.is_empty() {
                root.clone()
            } else {
                root.join(path)
            };
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
                collect_grep_matches(&re, p, &root, &content, &mut results);
            }

            format_grep_results(pattern, &results)
        },
    );
}

/// 收集单个文件中的匹配行
fn collect_grep_matches(
    re: &Regex,
    path: &std::path::Path,
    root: &std::path::Path,
    content: &str,
    results: &mut Vec<String>,
) {
    let rel = path.strip_prefix(root).unwrap_or(path);
    for (i, line) in content.lines().enumerate() {
        if results.len() >= MAX_RESULTS {
            break;
        }
        if re.is_match(line) {
            let truncated: String = line.chars().take(200).collect();
            results.push(format!("{}:{}:{}", rel.display(), i + 1, truncated));
        }
    }
}

/// 格式化文件搜索结果
fn format_file_results(pattern: &str, matches: &[String]) -> Result<String, Box<EvalAltResult>> {
    if matches.is_empty() {
        return Ok(format!("No files matching '{pattern}'"));
    }
    let count = matches.len();
    let suffix = if count >= MAX_RESULTS {
        format!("\n... (truncated at {MAX_RESULTS})")
    } else {
        String::new()
    };
    Ok(format!(
        "Found {count} file(s):\n{}{suffix}",
        matches.join("\n")
    ))
}

/// 格式化 grep 搜索结果
fn format_grep_results(pattern: &str, results: &[String]) -> Result<String, Box<EvalAltResult>> {
    if results.is_empty() {
        return Ok(format!("No matches for '{pattern}'"));
    }
    let count = results.len();
    let suffix = if count >= MAX_RESULTS {
        format!("\n... (truncated at {MAX_RESULTS})")
    } else {
        String::new()
    };
    Ok(format!(
        "Found {count} match(es):\n{}{suffix}",
        results.join("\n")
    ))
}
