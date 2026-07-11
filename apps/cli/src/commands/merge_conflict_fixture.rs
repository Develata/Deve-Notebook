//! Deterministic merge-conflict fixture for browser smoke validation.
//! plan_ref:
//!   - 05_diff_logic#merge-contract
//!   - 14_commands#cli-commands

use anyhow::{Result, bail};
use deve_core::ledger::merge::MergeResult;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{FactActor, LedgerEntry, MergeResolution, Op, PeerId, StructureOp};
use std::path::Path;

pub struct MergeConflictFixtureOptions {
    pub peer: String,
    pub repo: Option<String>,
    pub path: String,
    pub base: String,
    pub local: String,
    pub remote: String,
}

pub fn run(
    ledger_dir: &Path,
    snapshot_depth: usize,
    options: MergeConflictFixtureOptions,
) -> Result<()> {
    validate_fixture_path(&options.path)?;
    if options.base == options.local
        || options.base == options.remote
        || options.local == options.remote
    {
        bail!("merge conflict fixture requires three distinct contents");
    }

    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_name = resolve_repo_name(&repo, options.repo.as_deref())?;
    let repo_info = repo
        .get_repo_info_for(None, Some(&repo_name))?
        .ok_or_else(|| anyhow::anyhow!("Local repo metadata missing for {repo_name}"))?;
    let peer_id = PeerId::new(&options.peer);
    repo.ensure_shadow_repo_info(&peer_id, &shadow_info_for(&repo_info))?;

    let (doc_id, structure_ops) =
        repo.apply_file_structure_in_local_repo(&repo_name, &options.path, None, "fixture")?;
    let remote_structure_waterline =
        append_remote_structure(&repo, &peer_id, &repo_info.uuid, &structure_ops)?;
    let remote_base_seq = append_shared_base(
        &repo,
        &repo_name,
        &peer_id,
        &repo_info.uuid,
        doc_id,
        &options.base,
        remote_structure_waterline + 1,
    )?;
    establish_equal_checkpoint(&repo, &repo_name, &peer_id, &repo_info.uuid, doc_id)?;
    append_local_replace(&repo, &repo_name, doc_id, &options.base, &options.local)?;
    append_remote_replace(
        &repo,
        &peer_id,
        &repo_info.uuid,
        doc_id,
        &options.base,
        &options.remote,
        remote_base_seq + 1,
    )?;
    let workspace_root = repo.ensure_local_repo_workspace_identity(&repo_name)?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&workspace_root)?;
    let target = repo.local_repo_workspace_path(&repo_name, &options.path)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, &options.local)?;

    println!("merge_conflict_fixture: ok");
    println!("repo={repo_name}");
    println!("peer={peer_id}");
    println!("path={}", options.path);
    println!("doc_id={doc_id}");
    Ok(())
}

fn establish_equal_checkpoint(
    repo: &RepoManager,
    repo_name: &str,
    peer_id: &PeerId,
    repo_id: &uuid::Uuid,
    doc_id: deve_core::models::DocId,
) -> Result<()> {
    let evaluation = repo.merge_peer_in_local_repo(repo_name, peer_id, repo_id, doc_id)?;
    let MergeResult::Success(content) = evaluation.result else {
        bail!("equal fixture baseline unexpectedly produced a conflict");
    };
    repo.commit_peer_merge_in_local_repo(
        repo_name,
        &evaluation.preflight,
        &content,
        MergeResolution::EstablishEqual,
    )?;
    Ok(())
}

fn resolve_repo_name(repo: &RepoManager, requested: Option<&str>) -> Result<String> {
    let repo_name = requested.unwrap_or(repo.local_repo_name());
    if repo.get_repo_info_for(None, Some(repo_name))?.is_none() {
        bail!("Local repo not found: {repo_name}");
    }
    Ok(repo_name.to_string())
}

fn validate_fixture_path(path: &str) -> Result<()> {
    if path.trim().is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !path.ends_with(".md")
    {
        bail!("fixture path must be a safe repo-relative .md path");
    }
    Ok(())
}

fn shadow_info_for(local: &RepoInfo) -> RepoInfo {
    RepoInfo {
        uuid: local.uuid,
        name: local.name.clone(),
        url: local.url.clone(),
    }
}

fn append_remote_structure(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: &uuid::Uuid,
    ops: &[StructureOp],
) -> Result<u64> {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let entries = ops
        .iter()
        .enumerate()
        .map(|(idx, op)| {
            LedgerEntry::new_structure(op.clone(), timestamp, peer_id.clone(), idx as u64 + 1)
        })
        .collect::<Vec<_>>();
    repo.append_remote_ops(peer_id, repo_id, &entries)?;
    Ok(entries.len() as u64)
}

fn append_shared_base(
    repo: &RepoManager,
    repo_name: &str,
    peer_id: &PeerId,
    repo_id: &uuid::Uuid,
    doc_id: deve_core::models::DocId,
    base: &str,
    remote_seq: u64,
) -> Result<u64> {
    let timestamp = chrono::Utc::now().timestamp_millis();
    repo.local_fact_writer(FactActor::new("merge_fixture")?)
        .append_content_in_local_repo(
            repo_name,
            doc_id,
            Op::Insert {
                pos: 0,
                content: base.into(),
            },
            timestamp,
        )?;
    repo.append_remote_op(
        peer_id,
        repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: base.into(),
            },
            timestamp,
            peer_id.clone(),
            remote_seq,
            None,
            None,
        ),
    )?;
    Ok(remote_seq)
}

fn append_local_replace(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: deve_core::models::DocId,
    before: &str,
    after: &str,
) -> Result<()> {
    let writer = repo.local_fact_writer(FactActor::new("merge_fixture")?);
    writer.append_content_in_local_repo(
        repo_name,
        doc_id,
        Op::Delete {
            pos: 0,
            len: utf16_len(before),
        },
        chrono::Utc::now().timestamp_millis(),
    )?;
    writer.append_content_in_local_repo(
        repo_name,
        doc_id,
        Op::Insert {
            pos: 0,
            content: after.into(),
        },
        chrono::Utc::now().timestamp_millis(),
    )?;
    Ok(())
}

fn append_remote_replace(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: &uuid::Uuid,
    doc_id: deve_core::models::DocId,
    before: &str,
    after: &str,
    first_seq: u64,
) -> Result<()> {
    repo.append_remote_op(
        peer_id,
        repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Delete {
                pos: 0,
                len: utf16_len(before),
            },
            chrono::Utc::now().timestamp_millis(),
            peer_id.clone(),
            first_seq,
            None,
            None,
        ),
    )?;
    repo.append_remote_op(
        peer_id,
        repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: after.into(),
            },
            chrono::Utc::now().timestamp_millis(),
            peer_id.clone(),
            first_seq + 1,
            None,
            None,
        ),
    )?;
    Ok(())
}

fn utf16_len(value: &str) -> u32 {
    value.encode_utf16().count() as u32
}

#[cfg(test)]
mod tests;
