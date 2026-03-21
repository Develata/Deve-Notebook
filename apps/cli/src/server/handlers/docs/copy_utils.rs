// apps/cli/src/server/handlers/docs/copy_utils.rs
//! # 复制辅助函数
//!
//! **功能**: 提供迭代式目录复制和批量 Ledger 注册。
//! **低资源考量**: 采用流式复制 + 显式栈，避免栈溢出和内存膨胀。

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
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_copy_dir_assets_only_skips_markdown() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src_dir");
        let dst = tmp.path().join("dst_dir");

        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.md"), "content a").unwrap();
        fs::write(src.join("a.txt"), "asset a").unwrap();
        fs::write(src.join("sub/b.md"), "content b").unwrap();
        fs::write(src.join("sub/c.png"), "asset c").unwrap();

        copy_dir_assets_only(&src, &dst).unwrap();

        assert!(!dst.join("a.md").exists());
        assert!(!dst.join("sub/b.md").exists());
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "asset a");
        assert_eq!(
            fs::read_to_string(dst.join("sub/c.png")).unwrap(),
            "asset c"
        );
    }

    #[test]
    fn test_collect_md_files() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("vault");

        fs::create_dir_all(dir.join("notes")).unwrap();
        fs::write(dir.join("index.md"), "").unwrap();
        fs::write(dir.join("notes/daily.md"), "").unwrap();
        fs::write(dir.join("notes/ignore.txt"), "").unwrap();

        let files = collect_md_files(&dir, &dir).unwrap();
        assert!(files.contains(&"index.md".to_string()));
        assert!(files.contains(&"notes/daily.md".to_string()));
        assert!(!files.iter().any(|f| f.ends_with(".txt")));
    }

    #[test]
    fn test_collect_md_files_fails_closed_when_file_escapes_base() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("vault");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.md"), "").unwrap();

        let err = collect_md_files(&dir, &tmp.path().join("other"))
            .expect_err("base mismatch must fail closed");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_collect_dirs_fails_closed_when_dir_escapes_base() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("vault");
        fs::create_dir_all(dir.join("notes")).unwrap();

        let err =
            collect_dirs(&dir, &tmp.path().join("other")).expect_err("base mismatch must fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_deep_directory_no_stack_overflow() {
        let tmp = tempdir().unwrap();
        let mut current = tmp.path().to_path_buf();

        for i in 0..100 {
            current = current.join(format!("level_{}", i));
        }
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("deep.md"), "deep content").unwrap();
        fs::write(current.join("deep.bin"), "deep asset").unwrap();

        let dst = tmp.path().join("dst");
        copy_dir_assets_only(&tmp.path().join("level_0"), &dst).unwrap();

        let mut check_path = dst.clone();
        for i in 1..100 {
            check_path = check_path.join(format!("level_{}", i));
        }
        assert!(!check_path.join("deep.md").exists());
        assert!(check_path.join("deep.bin").exists());
    }
}
