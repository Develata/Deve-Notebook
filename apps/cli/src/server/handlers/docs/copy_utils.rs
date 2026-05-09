//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Iterative copy helpers for docs copy operations.

use deve_core::utils::path::path_to_forward_slash;
use std::io;
use std::path::{Path, PathBuf};

/// 迭代式复制目录中的非 Markdown 资产 (避免栈溢出)
///
/// **不变量 (Invariants)**:
/// - 源目录必须存在且为目录
/// - 目标目录在复制前不存在
/// - `.md` 文件必须由 Ledger 重建，不能直接把工作区文件当真值复制
///
/// **实现**: 使用显式栈代替递归，O(depth) 堆内存，无栈溢出风险
///
/// **复杂度**: O(n) 其中 n 为文件总数，栈深度 O(max_depth)
pub fn copy_dir_assets_only(src: &Path, dst: &Path) -> io::Result<()> {
    let mut stack: Vec<(PathBuf, PathBuf)> = vec![(src.to_path_buf(), dst.to_path_buf())];

    while let Some((src_dir, dst_dir)) = stack.pop() {
        std::fs::create_dir_all(&dst_dir)?;

        for entry in std::fs::read_dir(&src_dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst_dir.join(entry.file_name());

            if file_type.is_dir() {
                stack.push((src_path, dst_path));
            } else if file_type.is_file() && !is_markdown_path(&src_path) {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
    }

    Ok(())
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
}

/// 收集目录下所有 `.md` 文件的相对路径 (迭代式)
///
/// **用途**: 批量注册 Ledger DocId
///
/// **参数**:
/// - `dir`: 目标目录绝对路径
/// - `base`: 基准路径 (用于计算相对路径)
///
/// **返回**: 相对于 `base` 的 `.md` 文件路径列表 (正斜杠格式)
pub fn collect_md_files(dir: &Path, base: &Path) -> io::Result<Vec<String>> {
    let mut results = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        for entry in std::fs::read_dir(&current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file()
                && let Some(ext) = path.extension()
                && ext == "md"
            {
                results.push(relative_path_under_base(base, &path)?);
            }
        }
    }

    Ok(results)
}

/// 收集目录下所有子目录的相对路径 (包含根目录)
pub fn collect_dirs(dir: &Path, base: &Path) -> io::Result<Vec<String>> {
    let mut results = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        let rel_str = relative_path_under_base(base, &current_dir)?;
        if !rel_str.is_empty() {
            results.push(rel_str);
        }

        for entry in std::fs::read_dir(&current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(results)
}

fn relative_path_under_base(base: &Path, path: &Path) -> io::Result<String> {
    let rel = path.strip_prefix(base).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("docs copy traversal escaped base {:?}: {:?}", base, path),
        )
    })?;
    Ok(path_to_forward_slash(rel))
}

#[cfg(test)]
mod tests;
