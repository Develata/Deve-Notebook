// crates/core/src/ledger/manager/ops_ops.rs
//! # 操作日志追加/读取
//!
//! 实现 `RepoManager` 的操作追加和读取方法。

use crate::ledger::RepoManager;
use crate::ledger::node_ops;
use crate::ledger::ops;
use crate::models::{DocId, LedgerEntry, PeerId, RepoType, StructureOp};
use anyhow::Result;

impl RepoManager {
    pub fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        ops::append_op_to_db(&self.local_db, entry)
    }

    pub fn append_local_op_in_local_repo(
        &self,
        repo_name: &str,
        entry: &LedgerEntry,
    ) -> Result<u64> {
        self.run_on_local_repo(repo_name, |db| ops::append_op_to_db(db, entry))
    }

    pub fn append_generated_op(
        &self,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        ops::append_generated_op(&self.local_db, doc_id, peer_id, op_entry_builder)
    }

    pub fn append_generated_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.run_on_local_repo(repo_name, move |db| {
            ops::append_generated_op(db, doc_id, peer_id, op_entry_builder)
        })
    }

    pub fn append_generated_client_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        client_id: u64,
        client_op_id: u64,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.run_on_local_repo(repo_name, move |db| {
            ops::append_generated_client_op(
                db,
                doc_id,
                peer_id,
                client_id,
                client_op_id,
                op_entry_builder,
            )
        })
    }

    pub fn append_generated_structure_event_in_local_repo(
        &self,
        repo_name: &str,
        peer_id: PeerId,
        structure: StructureOp,
        timestamp: i64,
    ) -> Result<(u64, u64)> {
        self.run_on_local_repo(repo_name, move |db| {
            node_ops::append_generated_structure_op(db, peer_id, structure, timestamp)
        })
    }

    pub fn get_ops(&self, repo_type: &RepoType, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_repo_db(repo_type, |db| ops::get_ops_from_db(db, doc_id))
    }

    pub fn get_local_ops(&self, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        ops::get_ops_from_db(&self.local_db, doc_id)
    }

    pub fn get_local_ops_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_local_repo(repo_name, |db| ops::get_ops_from_db(db, doc_id))
    }

    pub fn find_client_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        client_id: u64,
        client_op_id: u64,
    ) -> Result<Option<(u64, LedgerEntry)>> {
        self.run_on_local_repo(repo_name, |db| {
            ops::find_client_op_in_db(db, doc_id, client_id, client_op_id)
        })
    }
}
