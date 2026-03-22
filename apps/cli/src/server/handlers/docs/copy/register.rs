use super::super::errors;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::copy_utils::{collect_dirs, collect_md_files};
use crate::server::handlers::docs::file_register::create_file_from_content;
use crate::server::repo_scope::{ResolvedRepo, local_repo_root};
use anyhow::{Result, anyhow};
use deve_core::state;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) struct CopyRegisterCtx<'a> {
    pub state: &'a Arc<AppState>,
    pub ch: &'a DualChannel,
    pub scope: &'a ResolvedRepo,
    pub scope_nonce: Option<u64>,
}

pub(super) fn register_copied_docs(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let base = match local_repo_root(ctx.state, ctx.scope) {
        Ok(path) => path,
        Err(err) => {
            errors::classified_failure_scoped(ctx.ch, err.to_string(), ctx.scope_nonce);
            return false;
        }
    };
    register_dirs(ctx, src, &base, src_path, dest_path)
        && register_files(ctx, src, &base, src_path, dest_path)
}

fn register_dirs(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    base: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let mut dirs = match collect_dirs(src, base) {
        Ok(dirs) => dirs,
        Err(err) => {
            tracing::error!("收集目录失败: {:?}", err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to collect copied dirs: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    dirs.sort_by_key(|path| path.matches('/').count());
    for dir_path in dirs {
        let dest_rel = match map_dest_rel(&dir_path, src_path, dest_path) {
            Ok(dest_rel) => dest_rel,
            Err(err) => {
                errors::storage_persist_failed_scoped(
                    ctx.ch,
                    format!("Failed to map copied dir path: {}", err),
                    ctx.scope_nonce,
                );
                return false;
            }
        };
        match ctx.state.repo.apply_dir_create_structure_in_local_repo(
            &ctx.scope.repo_name,
            &dest_rel,
            "local_copy",
        ) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("目录节点创建失败: {:?}", e);
                errors::storage_persist_failed_scoped(
                    ctx.ch,
                    format!("Dir node creation failed: {}", e),
                    ctx.scope_nonce,
                );
                return false;
            }
        }
    }
    true
}

fn register_files(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    base: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    match collect_md_files(src, base) {
        Ok(files) => {
            let count = files.len();
            for rel_path in files {
                let dest_rel = match map_dest_rel(&rel_path, src_path, dest_path) {
                    Ok(dest_rel) => dest_rel,
                    Err(err) => {
                        errors::storage_persist_failed_scoped(
                            ctx.ch,
                            format!("Failed to map copied file path: {}", err),
                            ctx.scope_nonce,
                        );
                        return false;
                    }
                };
                if !register_file(ctx, &rel_path, &dest_rel) {
                    return false;
                }
            }
            tracing::info!("目录复制完成: {} 下注册 {} 个文档", dest_path, count);
            true
        }
        Err(e) => {
            tracing::error!("收集 .md 文件失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to collect copied files: {}", e),
                ctx.scope_nonce,
            );
            false
        }
    }
}

fn register_file(ctx: CopyRegisterCtx<'_>, src_rel: &str, dest_rel: &str) -> bool {
    let doc_id = match ctx
        .state
        .repo
        .get_tracked_docid_in_local_repo(&ctx.scope.repo_name, src_rel)
    {
        Ok(Some(doc_id)) => doc_id,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ctx.ch,
                format!("Source doc not tracked: {}", src_rel),
                ctx.scope_nonce,
            );
            return false;
        }
        Err(err) => {
            errors::classified_failure_scoped(
                ctx.ch,
                format!("Failed to resolve copied source: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    let content = match ctx
        .state
        .repo
        .get_local_ops_in_local_repo(&ctx.scope.repo_name, doc_id)
    {
        Ok(ops) => {
            let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
            state::reconstruct_content(&entries)
        }
        Err(err) => {
            tracing::error!("加载复制源失败 {}: {:?}", src_rel, err);
            errors::storage_persist_failed_scoped(
                ctx.ch,
                format!("Failed to load copied file: {}", err),
                ctx.scope_nonce,
            );
            return false;
        }
    };
    let doc_id =
        match create_file_from_content(ctx.state, ctx.scope, dest_rel, &content, "local_copy") {
            Ok((doc_id, _ops)) => doc_id,
            Err(err) => {
                tracing::error!("Ledger 注册失败 {}: {:?}", dest_rel, err);
                errors::storage_persist_failed_scoped(
                    ctx.ch,
                    format!("Failed to register copied file: {}", err),
                    ctx.scope_nonce,
                );
                return false;
            }
        };
    tracing::debug!(
        "注册复制文档: {} -> {} (DocId: {})",
        src_rel,
        dest_rel,
        doc_id
    );
    true
}

fn map_dest_rel(src_rel: &str, src_path: &str, dest_path: &str) -> Result<String> {
    let suffix = src_rel.strip_prefix(src_path).ok_or_else(|| {
        anyhow!(
            "Copied path {} is not under source root {}",
            src_rel,
            src_path
        )
    })?;
    let trimmed = suffix.trim_start_matches('/');
    if trimmed.is_empty() {
        return Ok(dest_path.to_string());
    }
    Ok(format!("{}/{}", dest_path.trim_end_matches('/'), trimmed))
}

#[cfg(test)]
mod tests {
    use super::map_dest_rel;

    #[test]
    fn map_dest_rel_fails_closed_when_source_rel_escapes_root() {
        let err = map_dest_rel("notes/b.md", "other", "dest")
            .expect_err("path outside source root must fail closed");
        assert!(err.to_string().contains("is not under source root"));
    }

    #[test]
    fn map_dest_rel_preserves_exact_root_copy() {
        assert_eq!(
            map_dest_rel("notes", "notes", "dest").expect("root copy maps"),
            "dest"
        );
    }
}
