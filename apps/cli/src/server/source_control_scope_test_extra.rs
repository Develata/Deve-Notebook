//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Source-control remote scope tests.

use super::support::recv_changes;
use super::*;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{DocId, LedgerEntry, NodeId, Op, PeerId, RepoId, StructureOp};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::source_control::commits::{self, COMMITS_ORDER_TABLE};
use deve_core::source_control::{ChangeStatus, CommitInfo};

fn ensure_shadow_repo(repo: &RepoManager, repo_id: RepoId) -> anyhow::Result<PeerId> {
    let peer_id = PeerId::new("peer-a");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;
    Ok(peer_id)
}

fn shadow_create_file(peer_id: &PeerId, doc_id: DocId, name: &str, timestamp: i64) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::CreateFile {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id,
            parent_id: None,
            name: name.into(),
        },
        timestamp,
        peer_id.clone(),
        timestamp as u64,
    )
}

fn shadow_insert(
    peer_id: &PeerId,
    doc_id: DocId,
    pos: u32,
    content: &str,
    timestamp: i64,
) -> LedgerEntry {
    LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos,
            content: content.into(),
        },
        timestamp,
        peer_id.clone(),
        timestamp as u64,
        None,
        None,
    )
}

fn shadow_rename(peer_id: &PeerId, doc_id: DocId, new_name: &str, timestamp: i64) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::RenameNode {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id: Some(doc_id),
            new_name: new_name.into(),
        },
        timestamp,
        peer_id.clone(),
        timestamp as u64,
    )
}

fn create_shadow_commit(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: &RepoId,
    message: &str,
    ledger_seq: u64,
) -> anyhow::Result<CommitInfo> {
    repo.run_on_shadow_repo_by_id(peer_id, repo_id, |db| {
        commits::create(db, message, 1, ledger_seq)
    })
}

#[path = "source_control_scope_doc_diff_test_extra.rs"]
mod doc_diff;

#[path = "source_control_scope_history_test_extra.rs"]
mod history;
