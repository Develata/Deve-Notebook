// apps/cli/src/commands/seed.rs
//! # Seed 命令
//!
//! 将 Local Repo 的所有操作复制到指定 Peer 的 Shadow Repo。
//!
//! ## ⚠️ 警告
//! 此命令仅用于 **测试** 或 **手动迁移** 场景。
//! 它会将本地数据"伪装"成来自 `target_peer` 的数据写入 Shadow 库。
//! **生产环境误用可能导致数据混淆！**

use crate::commands::repo_arg::resolve_local_repo_arg;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use std::path::PathBuf;

/// Seed 命令入口
///
/// 将 Local Repo 的操作复制到 Shadow Repo 以模拟远端数据。
pub fn run(
    ledger_dir: &PathBuf,
    target_peer: String,
    repo_name: Option<String>,
    snapshot_depth: usize,
) -> Result<()> {
    tracing::info!("Starting Seed Command...");
    tracing::info!("Ledger Dir: {:?}", ledger_dir);
    tracing::info!("Target Peer: {}", target_peer);

    // ⚠️ 安全警告
    tracing::warn!("======================================================");
    tracing::warn!("⚠️  警告: 此命令会修改 Shadow Repo (remotes/)");
    tracing::warn!("⚠️  用途: 仅限测试或手动迁移，生产环境请勿误用!");
    tracing::warn!("======================================================");

    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_name = resolve_local_repo_arg(&repo, repo_name.as_deref())?;
    let peer_id = PeerId::new(&target_peer);

    let repo_info = repo
        .get_repo_info_for(None, Some(&repo_name))?
        .ok_or_else(|| anyhow::anyhow!("Local repo metadata missing"))?;
    repo.ensure_shadow_repo_info(&peer_id, &repo_info)?;
    let repo_id = repo_info.uuid;

    // 1. 列出所有本地文档
    let docs = repo.list_local_docs(Some(&repo_name))?;
    tracing::info!("找到 {} 个本地文档待 Seed。", docs.len());

    let mut total_ops = 0;

    for (doc_id, path) in docs {
        tracing::info!("Seeding 文档: {} ({})", path, doc_id);

        // 2. 获取本地操作
        let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;

        // 3. 写入远端 Shadow
        for (_, entry) in ops {
            repo.append_remote_op(&peer_id, &repo_id, &entry)?;
            total_ops += 1;
        }
    }

    tracing::info!("✅ Seed 完成。");
    tracing::info!("共复制 {} 个 Ops", total_ops);
    tracing::info!(
        "Shadow Repo: remotes/{}/{}.redb",
        target_peer,
        repo_info.name
    );

    Ok(())
}
