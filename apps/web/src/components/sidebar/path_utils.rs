// apps/web/src/components/sidebar/path_utils.rs
//! plan_ref:
//!   - 04_storage#internal-path-normalization
//!   - 06_repository#tree-projection-contract
//!
//! # 路径工具函数
//!
//! 提供文件路径处理工具，如自动重命名以避免冲突。

#![allow(dead_code)] // 为文件操作功能预留

use deve_core::models::DocId;

/// 查找可用的目标路径名
///
/// 如果目标路径已存在，自动生成带后缀的新名称。
///
/// **命名规则**:
/// - 原名: `note.md` -> `note copy.md` -> `note copy 2.md` -> ...
/// - 目录: `folder` -> `folder copy` -> `folder copy 2` -> ...
///
/// **参数**:
/// - `base_path`: 初始目标路径 (如 `notes/daily.md`)
/// - `existing_paths`: 已存在的路径列表
///
/// **返回**: 不冲突的路径
pub fn find_available_path(base_path: &str, existing_paths: &[(DocId, String)]) -> String {
    let exists = |p: &str| existing_paths.iter().any(|(_, path)| path == p);

    if !exists(base_path) {
        return base_path.to_string();
    }

    // 分离文件名和扩展名
    let (stem, ext) = split_path_ext(base_path);

    // 尝试 "stem copy.ext"
    let copy_path = format_with_suffix(stem, "copy", ext);
    if !exists(&copy_path) {
        return copy_path;
    }

    // 尝试 "stem copy N.ext" (N = 2, 3, ...)
    for n in 2..100 {
        let numbered_path = format_with_suffix(stem, &format!("copy {}", n), ext);
        if !exists(&numbered_path) {
            return numbered_path;
        }
    }

    // 回退: 使用时间戳
    #[cfg(target_arch = "wasm32")]
    let ts = js_sys::Date::now() as u64;
    #[cfg(not(target_arch = "wasm32"))]
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format_with_suffix(stem, &format!("copy {}", ts), ext)
}

/// 分离路径的基础名和扩展名
///
/// **示例**:
/// - `notes/daily.md` -> (`notes/daily`, `.md`)
/// - `folder` -> (`folder`, ``)
fn split_path_ext(path: &str) -> (&str, &str) {
    // 找到最后一个 `/` 后的文件名部分
    let file_name_start = path.rfind('/').map(|i| i + 1).unwrap_or(0);
    let file_name = &path[file_name_start..];

    // 在文件名中找扩展名 (仅当存在 `.` 且不在开头时)
    if let Some(dot_pos) = file_name.rfind('.')
        && dot_pos > 0
    {
        let ext_start = file_name_start + dot_pos;
        return (&path[..ext_start], &path[ext_start..]);
    }

    (path, "")
}

/// 格式化带后缀的路径
fn format_with_suffix(stem: &str, suffix: &str, ext: &str) -> String {
    format!("{} {}{}", stem, suffix, ext)
}

#[cfg(test)]
#[path = "path_utils_test.rs"]
mod tests;
