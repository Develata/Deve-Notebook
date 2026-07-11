// crates/core/src/ledger/manager/ops_ops.rs
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!
//! # 操作日志追加/读取
//!
//! 实现 `RepoManager` 的操作追加和读取方法。

use crate::ledger::RepoManager;
use crate::ledger::manager::structure_projection;
use crate::ledger::node_ops;
use crate::ledger::ops;
use crate::ledger::runtime_tables;
use crate::models::{DocId, FactActor, LedgerEntry, PeerId, RepoType, StructureOp};
use anyhow::Result;

impl RepoManager {
    pub fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        self.authority_storage_runtime().append_local_op(entry)
    }

    pub fn append_local_op_in_local_repo(
        &self,
        repo_name: &str,
        entry: &LedgerEntry,
    ) -> Result<u64> {
        self.authority_storage_runtime()
            .append_local_op_in_local_repo(repo_name, entry)
    }

    pub fn append_generated_op(
        &self,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.authority_storage_runtime()
            .append_generated_op(doc_id, peer_id, op_entry_builder)
    }

    pub fn append_generated_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.authority_storage_runtime()
            .append_generated_op_in_local_repo(repo_name, doc_id, peer_id, op_entry_builder)
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
        self.authority_storage_runtime()
            .append_generated_client_op_in_local_repo(
                repo_name,
                doc_id,
                peer_id,
                client_id,
                client_op_id,
                op_entry_builder,
            )
    }

    pub fn repair_client_op_index_if_missing_in_local_repo(&self, repo_name: &str) -> Result<bool> {
        self.run_on_local_repo(repo_name, runtime_tables::repair_client_op_index_if_missing)
    }

    pub fn append_generated_structure_event_in_local_repo(
        &self,
        repo_name: &str,
        peer_id: PeerId,
        structure: StructureOp,
        timestamp: i64,
    ) -> Result<(u64, u64)> {
        if &peer_id != self.local_peer_id() {
            anyhow::bail!(
                "Local structure fact origin {} does not match host identity {}",
                peer_id,
                self.local_peer_id()
            );
        }
        let repo_scope = ops::local_repo_scope(repo_name);
        self.run_on_local_repo(repo_name, move |db| {
            let write_txn = db.begin_write()?;
            let seqs = node_ops::append_generated_structure_op_to_txn(
                &write_txn,
                peer_id,
                FactActor::system(),
                structure.clone(),
                timestamp,
                &repo_scope,
            )?;
            structure_projection::apply_in_txn(&write_txn, &structure)?;
            write_txn.commit()?;
            Ok(seqs)
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
        client_id: u64,
        client_op_id: u64,
    ) -> Result<Option<(u64, LedgerEntry)>> {
        self.run_on_local_repo(repo_name, |db| {
            ops::find_client_op_in_db(db, client_id, client_op_id)
        })
    }
}
