//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Iterative copy helpers for docs copy operations.

use deve_core::utils::path::path_to_forward_slash;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) struct PreparedAssetCopy {
    source: PathBuf,
    destination: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
}

pub(super) fn prepare_dir_asset_copies(
    src: &Path,
    dst: &Path,
) -> io::Result<Vec<PreparedAssetCopy>> {
    let mut assets = Vec::new();
    let mut stack = vec![src.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let source = entry.path();
            if file_type.is_dir() {
                stack.push(source);
                continue;
            }
            if !file_type.is_file() || is_markdown_path(&source) {
                continue;
            }
            let relative = source
                .strip_prefix(src)
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("asset copy source escaped root: {}", source.display()),
                    )
                })?
                .to_path_buf();
            let metadata = entry.metadata()?;
            assets.push(PreparedAssetCopy {
                source,
                destination: dst.join(relative),
                len: metadata.len(),
                modified: metadata.modified().ok(),
            });
        }
    }
    assets.sort_by(|left, right| left.destination.cmp(&right.destination));
    Ok(assets)
}

pub(super) fn apply_prepared_asset_copies(assets: &[PreparedAssetCopy]) -> io::Result<()> {
    for asset in assets {
        let metadata = std::fs::metadata(&asset.source)?;
        if !metadata.is_file()
            || metadata.len() != asset.len
            || asset
                .modified
                .is_some_and(|expected| metadata.modified().ok() != Some(expected))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "asset copy source changed after preparation: {}",
                    asset.source.display()
                ),
            ));
        }
        if asset.destination.try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "asset copy destination appeared after preparation: {}",
                    asset.destination.display()
                ),
            ));
        }
        if let Some(parent) = asset.destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&asset.source, &asset.destination)?;
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
