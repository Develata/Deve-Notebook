//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 07_network#server-ws-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! Host-bound entry point for allocating and appending local content facts.

use crate::ledger::manager::types::RepoManager;
use crate::models::{DocId, FactActor, LedgerEntry, Op};
use anyhow::Result;

pub struct LocalFactWriter<'a> {
    manager: &'a RepoManager,
    actor: FactActor,
}

impl<'a> LocalFactWriter<'a> {
    pub(crate) fn new(manager: &'a RepoManager, actor: FactActor) -> Self {
        Self { manager, actor }
    }

    pub fn append_content_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        op: Op,
        timestamp: i64,
    ) -> Result<(u64, u64)> {
        self.append_content_with_client_in_local_repo(repo_name, doc_id, op, timestamp, None, None)
    }

    pub fn append_client_content_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        op: Op,
        timestamp: i64,
        client_id: u64,
        client_op_id: u64,
    ) -> Result<(u64, u64)> {
        self.append_content_with_client_in_local_repo(
            repo_name,
            doc_id,
            op,
            timestamp,
            Some(client_id),
            Some(client_op_id),
        )
    }

    fn append_content_with_client_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        op: Op,
        timestamp: i64,
        client_id: Option<u64>,
        client_op_id: Option<u64>,
    ) -> Result<(u64, u64)> {
        let origin_peer_id = self.manager.local_peer_id().clone();
        let actor = self.actor.clone();
        if let (Some(client_id), Some(client_op_id)) = (client_id, client_op_id) {
            return self
                .manager
                .authority_storage_runtime()
                .append_generated_client_op_in_local_repo(
                    repo_name,
                    doc_id,
                    origin_peer_id.clone(),
                    client_id,
                    client_op_id,
                    move |seq| {
                        LedgerEntry::new_content_with_actor(
                            doc_id,
                            op.clone(),
                            timestamp,
                            origin_peer_id.clone(),
                            seq.into(),
                            actor.clone(),
                            Some(client_id),
                            Some(client_op_id),
                        )
                    },
                );
        }

        self.manager
            .authority_storage_runtime()
            .append_generated_op_in_local_repo(
                repo_name,
                doc_id,
                origin_peer_id.clone(),
                move |seq| {
                    LedgerEntry::new_content_with_actor(
                        doc_id,
                        op.clone(),
                        timestamp,
                        origin_peer_id.clone(),
                        seq.into(),
                        actor.clone(),
                        None,
                        None,
                    )
                },
            )
    }
}

impl RepoManager {
    pub fn local_fact_writer(&self, actor: FactActor) -> LocalFactWriter<'_> {
        LocalFactWriter::new(self, actor)
    }
}
