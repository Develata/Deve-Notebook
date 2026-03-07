use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::copy_utils::{collect_dirs, collect_md_files};
use crate::server::handlers::docs::node_helpers::{broadcast_dir_chain, broadcast_parent_dirs};
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::models::{NodeId, NodeKind};
use deve_core::protocol::ServerMessage;
use std::path::Path;
use std::sync::Arc;

pub(super) fn register_copied_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    dest_path: &str,
) {
    let base = &state.vault_path;
    register_dirs(state, ch, scope, dst, base);
    register_files(state, ch, scope, dst, base, dest_path);
}

fn register_dirs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    base: &Path,
) {
    let Ok(dirs) = collect_dirs(dst, base) else {
        return;
    };
    for dir_path in dirs {
        let created = run_on_resolved_local_repo(state, scope, |db| {
            if let Some(existing) = node_meta::get_node_id(db, &dir_path)? {
                let meta = node_meta::get_node_meta(db, existing)?
                    .ok_or_else(|| anyhow!("Node meta missing"))?;
                if meta.kind != NodeKind::Dir {
                    return Err(anyhow!("Path is not a directory: {}", dir_path));
                }
                return Ok(Some(existing));
            }
            let node_id = node_meta::create_dir_node(db, &dir_path)?;
            Ok(Some(node_id))
        });

        match created {
            Ok(Some(node_id)) => {
                if let Err(e) =
                    broadcast_dir_chain(state, ch, scope.repo_id, &scope.repo_name, node_id)
                {
                    tracing::error!("广播目录链失败: {:?}", e);
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("目录节点创建失败: {:?}", e);
                ch.send_error(format!("Dir node creation failed: {}", e));
                return;
            }
        }
    }
}

fn register_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    base: &Path,
    dest_path: &str,
) {
    match collect_md_files(dst, base) {
        Ok(files) => {
            let count = files.len();
            for rel_path in files {
                register_file(state, ch, scope, &rel_path);
            }
            tracing::info!("目录复制完成: {} 下注册 {} 个文档", dest_path, count);
        }
        Err(e) => tracing::error!("收集 .md 文件失败: {:?}", e),
    }
}

fn register_file(state: &Arc<AppState>, ch: &DualChannel, scope: &ResolvedRepo, rel_path: &str) {
    let Ok(doc_id) = state
        .repo
        .create_docid_in_local_repo(&scope.repo_name, rel_path)
    else {
        tracing::warn!("Ledger 注册失败: {}", rel_path);
        return;
    };
    tracing::debug!("注册复制文档: {} (DocId: {})", rel_path, doc_id);
    let node_id = NodeId::from_doc_id(doc_id);
    let Ok(meta) = run_on_resolved_local_repo(state, scope, |db| {
        node_meta::get_node_meta(db, node_id)
            .and_then(|m| m.ok_or_else(|| anyhow!("File node meta missing")))
    }) else {
        return;
    };
    if let Err(e) =
        broadcast_parent_dirs(state, ch, scope.repo_id, &scope.repo_name, meta.parent_id)
    {
        tracing::error!("广播父目录失败: {:?}", e);
    }
    let delta = state.tree_manager.with_tree_mut(scope.repo_id, |tm| {
        tm.add_file(
            node_id,
            meta.path.clone(),
            meta.parent_id,
            meta.name.clone(),
            doc_id,
        )
    });
    ch.unicast(ServerMessage::TreeUpdate(delta));
}
