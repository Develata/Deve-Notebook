//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/projection#projection-contract
//!   - 10_rendering#document-authority-bridge

use crate::ledger::manager::types::RepoManager;
use crate::ledger::ops;
use crate::models::{DocId, LedgerEntry, PeerId};
use anyhow::Result;

pub(crate) struct AuthorityStorageRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> AuthorityStorageRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        self.ensure_local_origin(&entry.origin_peer_id)?;
        let repo_scope = ops::local_repo_scope(self.manager.local_repo_name());
        ops::append_op_to_db(&self.manager.local_db, entry, &repo_scope)
    }

    pub(crate) fn append_local_op_in_local_repo(
        &self,
        repo_name: &str,
        entry: &LedgerEntry,
    ) -> Result<u64> {
        self.ensure_local_origin(&entry.origin_peer_id)?;
        let repo_scope = ops::local_repo_scope(repo_name);
        self.manager
            .run_on_local_repo(repo_name, |db| ops::append_op_to_db(db, entry, &repo_scope))
    }

    pub(crate) fn append_generated_op(
        &self,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.ensure_local_origin(&peer_id)?;
        let repo_scope = ops::local_repo_scope(self.manager.local_repo_name());
        ops::append_generated_op(
            &self.manager.local_db,
            doc_id,
            peer_id,
            &repo_scope,
            op_entry_builder,
        )
    }

    pub(crate) fn append_generated_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.ensure_local_origin(&peer_id)?;
        let repo_scope = ops::local_repo_scope(repo_name);
        self.manager.run_on_local_repo(repo_name, move |db| {
            ops::append_generated_op(db, doc_id, peer_id, &repo_scope, op_entry_builder)
        })
    }

    pub(crate) fn append_generated_client_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        client_id: u64,
        client_op_id: u64,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.ensure_local_origin(&peer_id)?;
        let repo_scope = ops::local_repo_scope(repo_name);
        self.manager.run_on_local_repo(repo_name, move |db| {
            ops::append_generated_client_op(
                db,
                doc_id,
                peer_id,
                &repo_scope,
                client_id,
                client_op_id,
                op_entry_builder,
            )
        })
    }

    fn ensure_local_origin(&self, peer_id: &PeerId) -> Result<()> {
        if peer_id != self.manager.local_peer_id() {
            anyhow::bail!(
                "Local fact origin {} does not match host identity {}",
                peer_id,
                self.manager.local_peer_id()
            );
        }
        Ok(())
    }
}

impl RepoManager {
    pub(crate) fn authority_storage_runtime(&self) -> AuthorityStorageRuntime<'_> {
        AuthorityStorageRuntime::new(self)
    }
}
