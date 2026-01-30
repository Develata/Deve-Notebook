// crates\core\src\utils
//! # 跨平台路径工具
//!
//! 本模块提供跨平台文件路径处理工具。
//!
//! ## 设计原则
//!
//! 为保持一致性，所有存储在 Ledger 中的路径都使用**正斜杠格式**（Linux 风格）。
//! 这确保了在 Windows 和 Linux 之间的互操作性，并避免路径格式不一致的问题。

use std::path::{Path, PathBuf};

/// 将路径字符串转换为正斜杠格式（用于存储）。
///
/// 无论在什么操作系统上，都将反斜杠转换为正斜杠。
/// 这是存储路径到 Ledger 前必须调用的函数。
///
/// # Example
/// ```
/// use deve_core::utils::path::to_forward_slash;
/// assert_eq!(to_forward_slash("folder\\subfolder\\file.md"), "folder/subfolder/file.md");
/// assert_eq!(to_forward_slash("folder/subfolder/file.md"), "folder/subfolder/file.md");
/// ```
pub fn to_forward_slash(path: &str) -> String {
    path.replace('\\', "/")
}

/// 将路径转换为系统原生格式（用于文件系统操作）。
///
/// # Behavior
/// - On Windows: `foo/bar` becomes `foo\\bar`
/// - On Linux/macOS: path unchanged
///
/// 使用 `std::path::Path::components()` 迭代器重建路径，
/// 确保使用 OS 原生分隔符。
pub fn to_native(path: &str) -> String {
    // 使用 PathBuf::from() 解析路径组件，然后转回字符串
    // 这会自动使用 OS 原生分隔符重建路径
    let path_buf: PathBuf = Path::new(path).components().collect();
    path_buf.to_string_lossy().into_owned()
}

/// 规范化路径字符串（别名，保持向后兼容）。
///
/// **注意**：此函数现在始终返回正斜杠格式。
/// 如需系统原生格式，请使用 `to_native`。
pub fn normalize(path: &str) -> String {
    to_forward_slash(path)
}

/// 将 Path 对象转换为正斜杠格式的字符串。
pub fn path_to_forward_slash(path: &Path) -> String {
    to_forward_slash(&path.to_string_lossy())
}

/// Join a base path with a subpath, then convert to native format for FS operations.
///
/// 用于文件系统操作（创建、读取文件）。
pub fn join_normalized(base: &Path, subpath: &str) -> PathBuf {
    base.join(to_native(subpath))
}

/// Join a base path with a subpath, returning the path in forward-slash format.
///
/// 用于 Ledger 存储。
pub fn join_for_storage(base: &Path, subpath: &Path) -> String {
    let joined = base.join(subpath);
    path_to_forward_slash(&joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_forward_slash() {
        assert_eq!(
            to_forward_slash("folder\\subfolder\\file.md"),
            "folder/subfolder/file.md"
        );
        assert_eq!(
            to_forward_slash("folder/subfolder/file.md"),
            "folder/subfolder/file.md"
        );
        assert_eq!(to_forward_slash("a\\b/c\\d"), "a/b/c/d");
    }

    #[test]
    fn test_to_native() {
        let input = "folder/subfolder/file.md";
        let result = to_native(input);

        #[cfg(target_os = "windows")]
        assert_eq!(result, "folder\\subfolder\\file.md");

        #[cfg(not(target_os = "windows"))]
        assert_eq!(result, "folder/subfolder/file.md");
    }

    #[test]
    fn test_join_normalized() {
        let base = PathBuf::from("vault");
        let result = join_normalized(&base, "folder/file.md");

        #[cfg(target_os = "windows")]
        assert_eq!(result, PathBuf::from("vault\\folder\\file.md"));

        #[cfg(not(target_os = "windows"))]
        assert_eq!(result, PathBuf::from("vault/folder/file.md"));
    }

    #[test]
    fn test_normalize_always_forward_slash() {
        assert_eq!(normalize("a\\b\\c"), "a/b/c");
        assert_eq!(normalize("a/b/c"), "a/b/c");
    }
}
