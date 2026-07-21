//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 04_repository#repo-scope-runtime
//!   - 05_diff_logic#merge-contract
//!
//! Checkpoint-backed peer merge evaluation and atomic local commit.

use super::merge_ops_support::*;
use crate::codec;
use crate::ledger::merge::{
    MergeBaseCheckpoint, MergeCommitOutcome, MergeEngine, MergeEvaluation, MergePreflight,
    MergeResult,
};
use crate::ledger::schema::{MERGE_BASE_CHECKPOINT, PEER_FACT_SEQ};
use crate::ledger::{RepoManager, ops, reconcile};
use crate::models::{
    DocId, FactActor, LedgerEntry, MergeAnchor, MergeResolution, PeerFactSeq, PeerId, RepoId,
};
use anyhow::{Result, anyhow, bail};
use redb::ReadableTable;

impl RepoManager {
    pub fn merge_peer(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<MergeEvaluation> {
        let local_repo_name = self.current_local_repo_name()?;
        self.merge_peer_in_local_repo(&local_repo_name, peer_id, repo_id, doc_id)
    }

    /// Evaluate a source-isolated three-way merge without changing authority.
    pub fn merge_peer_in_local_repo(
        &self,
        repo_name: &str,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<MergeEvaluation> {
        if peer_id == self.local_peer_id() {
            bail!("merge source must not be the local physical peer: {peer_id}");
        }
        self.ensure_local_repo_scope(repo_name, repo_id)?;

        let _guard = self.lock_shadow_merge()?;
        self.evaluate_peer_merge_locked(repo_name, peer_id, repo_id, doc_id)
    }

    fn evaluate_peer_merge_locked(
        &self,
        repo_name: &str,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<MergeEvaluation> {
        let (local_waterline, local_entries) = self.read_stable_local_doc(repo_name, doc_id)?;
        let (source_waterline, source_entries) =
            self.read_stable_shadow_doc(peer_id, repo_id, doc_id)?;
        if local_entries.is_empty() {
            bail!("merge_local_doc_missing: doc={doc_id} repo={repo_id}");
        }
        if source_waterline == PeerFactSeq::ZERO || source_entries.is_empty() {
            bail!("merge_source_doc_missing: source={peer_id} repo={repo_id} doc={doc_id}");
        }
        ensure_source_entries(peer_id, &source_entries)?;

        let local_content = reconstruct(&local_entries);
        let source_content = reconstruct(&source_entries);
        let source_state_hash = hash_content(&source_content);
        let checkpoint =
            self.get_merge_base_checkpoint_in_local_repo(repo_name, peer_id, doc_id)?;

        let (base_content, expected_checkpoint_anchor_global_seq, establish_equal) =
            match checkpoint {
                None => {
                    if local_content != source_content {
                        bail!(
                            "merge_base_missing: source={} doc={} local/source already diverged",
                            peer_id,
                            doc_id
                        );
                    }
                    (source_content.clone(), None, true)
                }
                Some(checkpoint) => {
                    self.validate_merge_checkpoint(
                        repo_name,
                        peer_id,
                        repo_id,
                        doc_id,
                        &checkpoint,
                        source_waterline,
                        &source_entries,
                    )?;
                    let base = reconstruct_at(&source_entries, checkpoint.source_peer_seq);
                    (base, Some(checkpoint.anchor_global_seq), false)
                }
            };

        let result = MergeEngine::merge_commits(&base_content, &local_content, &source_content);
        let automatic_result = match &result {
            MergeResult::Success(content) => Some(content.clone()),
            MergeResult::Conflict { .. } => None,
        };
        Ok(MergeEvaluation {
            result,
            preflight: MergePreflight {
                source_peer_id: peer_id.clone(),
                repo_id: *repo_id,
                doc_id,
                expected_local_waterline: local_waterline,
                expected_source_waterline: source_waterline,
                expected_checkpoint_anchor_global_seq,
                local_content,
                source_content,
                source_state_hash,
                establish_equal,
                automatic_result,
            },
        })
    }

    /// Append content facts, MergeAnchor, and checkpoint in one local redb transaction.
    pub fn commit_peer_merge_in_local_repo(
        &self,
        repo_name: &str,
        preflight: &MergePreflight,
        target_content: &str,
        resolution: MergeResolution,
    ) -> Result<MergeCommitOutcome> {
        self.ensure_local_repo_scope(repo_name, &preflight.repo_id)?;
        let _guard = self.lock_shadow_merge()?;
        let reevaluated = self.evaluate_peer_merge_locked(
            repo_name,
            &preflight.source_peer_id,
            &preflight.repo_id,
            preflight.doc_id,
        )?;
        if reevaluated.preflight != *preflight {
            bail!("merge_preflight_invalid_or_stale: core evaluation evidence changed");
        }
        let preflight = &reevaluated.preflight;
        validate_resolution(preflight, target_content, resolution)?;

        let (source_waterline, source_entries) = self.read_stable_shadow_doc(
            &preflight.source_peer_id,
            &preflight.repo_id,
            preflight.doc_id,
        )?;
        ensure_source_entries(&preflight.source_peer_id, &source_entries)?;
        let source_content = reconstruct(&source_entries);
        if source_waterline != preflight.expected_source_waterline
            || source_content != preflight.source_content
            || hash_content(&source_content) != preflight.source_state_hash
        {
            bail!(
                "merge_source_drift: source={} expected_waterline={} observed_waterline={}",
                preflight.source_peer_id,
                preflight.expected_source_waterline,
                source_waterline
            );
        }

        let (local_waterline, local_entries) =
            self.read_stable_local_doc(repo_name, preflight.doc_id)?;
        let local_content = reconstruct(&local_entries);
        if local_waterline != preflight.expected_local_waterline
            || local_content != preflight.local_content
        {
            bail!(
                "merge_local_drift: expected_waterline={} observed_waterline={}",
                preflight.expected_local_waterline,
                local_waterline
            );
        }
        let patch = reconcile::compute_reconcile_patch(&local_entries, target_content)?;
        let local_peer_id = self.local_peer_id().clone();
        let repo_scope = ops::local_repo_scope(repo_name);
        let source_state_hash = preflight.source_state_hash;
        let result_hash = hash_content(target_content);

        self.run_on_local_repo(repo_name, |db| {
            let write_txn = db.begin_write()?;
            let current_local_waterline = {
                let table = write_txn.open_table(PEER_FACT_SEQ)?;
                table
                    .get(local_peer_id.as_str())?
                    .map(|value| PeerFactSeq::new(value.value()))
                    .unwrap_or(PeerFactSeq::ZERO)
            };
            if current_local_waterline != preflight.expected_local_waterline {
                bail!(
                    "merge_local_drift: expected_waterline={} observed_waterline={}",
                    preflight.expected_local_waterline,
                    current_local_waterline
                );
            }
            ensure_checkpoint_generation(
                &write_txn,
                &preflight.source_peer_id,
                preflight.doc_id,
                preflight.expected_checkpoint_anchor_global_seq,
            )?;

            reconcile::append_patch_to_txn(
                &write_txn,
                preflight.doc_id,
                &local_peer_id,
                "merge",
                &repo_scope,
                &patch,
            )?;
            let anchor_peer_seq =
                ops::write_direct::next_peer_fact_seq(&write_txn, &local_peer_id)?;
            let anchor = MergeAnchor {
                source_peer_id: preflight.source_peer_id.clone(),
                source_waterline: preflight.expected_source_waterline,
                local_pre_merge_waterline: preflight.expected_local_waterline,
                source_state_hash,
                result_hash,
                resolution,
            };
            let entry = LedgerEntry::new_merge_anchor_with_actor(
                preflight.doc_id,
                anchor,
                chrono::Utc::now().timestamp_millis(),
                local_peer_id.clone(),
                anchor_peer_seq,
                FactActor::new("merge")?,
            );
            let anchor_global_seq = ops::append_op_to_txn(&write_txn, &entry, &repo_scope)?;
            let checkpoint = MergeBaseCheckpoint {
                source_peer_id: preflight.source_peer_id.clone(),
                doc_id: preflight.doc_id,
                local_anchor_peer_seq: anchor_peer_seq,
                source_peer_seq: preflight.expected_source_waterline,
                source_state_hash,
                result_hash,
                anchor_global_seq,
            };
            let bytes = codec::encode(&checkpoint)?;
            write_txn.open_table(MERGE_BASE_CHECKPOINT)?.insert(
                (
                    preflight.source_peer_id.as_str(),
                    preflight.doc_id.as_u128(),
                ),
                bytes.as_slice(),
            )?;
            write_txn.commit()?;
            Ok(MergeCommitOutcome {
                content_changed: !patch.is_empty(),
                anchor_global_seq,
                anchor_peer_seq,
                resolution,
            })
        })
    }

    pub fn get_merge_base_checkpoint_in_local_repo(
        &self,
        repo_name: &str,
        source_peer_id: &PeerId,
        doc_id: DocId,
    ) -> Result<Option<MergeBaseCheckpoint>> {
        self.run_on_local_repo(repo_name, |db| {
            let read = db.begin_read()?;
            let table = read.open_table(MERGE_BASE_CHECKPOINT)?;
            let Some(bytes) = table.get((source_peer_id.as_str(), doc_id.as_u128()))? else {
                return Ok(None);
            };
            let checkpoint: MergeBaseCheckpoint = codec::decode(bytes.value())?;
            if checkpoint.source_peer_id != *source_peer_id || checkpoint.doc_id != doc_id {
                bail!(
                    "merge_checkpoint_key_mismatch: source={} doc={}",
                    source_peer_id,
                    doc_id
                );
            }
            Ok(Some(checkpoint))
        })
    }

    fn ensure_local_repo_scope(&self, repo_name: &str, repo_id: &RepoId) -> Result<()> {
        let observed = self.run_on_local_repo(repo_name, Self::read_repo_info_from_db)?;
        let observed =
            observed.ok_or_else(|| anyhow!("local repo metadata missing: {repo_name}"))?;
        if observed.uuid != *repo_id {
            bail!(
                "local repo scope mismatch: selector={} expected={} observed={}",
                repo_name,
                repo_id,
                observed.uuid
            );
        }
        Ok(())
    }

    fn read_stable_local_doc(
        &self,
        repo_name: &str,
        doc_id: DocId,
    ) -> Result<(PeerFactSeq, Vec<LedgerEntry>)> {
        self.run_on_local_repo(repo_name, |db| {
            stable_doc_snapshot(db, self.local_peer_id(), doc_id)
        })
    }

    fn read_stable_shadow_doc(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
    ) -> Result<(PeerFactSeq, Vec<LedgerEntry>)> {
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            stable_doc_snapshot(db, peer_id, doc_id)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_merge_checkpoint(
        &self,
        repo_name: &str,
        source_peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: DocId,
        checkpoint: &MergeBaseCheckpoint,
        source_waterline: PeerFactSeq,
        source_entries: &[LedgerEntry],
    ) -> Result<()> {
        if checkpoint.source_peer_seq == PeerFactSeq::ZERO
            || checkpoint.source_peer_seq > source_waterline
        {
            bail!(
                "merge_checkpoint_waterline_invalid: source={} checkpoint={} observed={}",
                source_peer_id,
                checkpoint.source_peer_seq,
                source_waterline
            );
        }
        let source_base = reconstruct_at(source_entries, checkpoint.source_peer_seq);
        if hash_content(&source_base) != checkpoint.source_state_hash {
            bail!(
                "merge_checkpoint_source_hash_mismatch: source={} doc={} repo={}",
                source_peer_id,
                doc_id,
                repo_id
            );
        }
        self.run_on_local_repo(repo_name, |db| {
            validate_anchor_reference(db, self.local_peer_id(), checkpoint)
        })
    }
}
