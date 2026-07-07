//! plan_ref:
//!   - 06_backup#backup-explicit-import-authority-path
//!   - 03_storage/authority#facts-partition
//!   - 04_repository#repo-scope-runtime
//!
//! Explicit backup import authority path.
//!
//! This runtime only admits a verified RestoreCandidate into an empty local
//! repository through the normal ledger append validator. It does not stage
//! Source Control changes, create commit anchors, enqueue Git mirror work, or
//! touch Projection Workspace files.

use crate::backup::restore::{plaintext_evidence_digest, verify_restore_candidate_fingerprint};
use crate::backup::{
    BackupDigest, BackupPackPlaintextError, BackupPlaintextPacksResult, BackupRestoreError,
    RestoreAdmissionState, RestoreCandidate,
};
use crate::ledger::manager::types::RepoManager;
use crate::ledger::ops;
use crate::ledger::schema::LEDGER_OPS;
use crate::models::RepoId;
use redb::ReadableTable;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct BackupRestoreImportInput<'a> {
    pub candidate: &'a RestoreCandidate,
    pub plaintext_packs: &'a BackupPlaintextPacksResult,
    pub expected_candidate_fingerprint: &'a BackupDigest,
    pub write_gate_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRestoreImportReport {
    pub repo_id: RepoId,
    pub repo_name: String,
    pub candidate_fingerprint: BackupDigest,
    pub imported_ledger_entries: u64,
    pub first_imported_seq: u64,
    pub last_imported_seq: u64,
}

#[derive(Debug, Error)]
pub enum BackupRestoreImportError {
    #[error("backup explicit import requires an explicit write gate")]
    WriteGateRequired,
    #[error("backup explicit import requires an ExplicitImport restore candidate")]
    CandidateNotExplicitImport,
    #[error("backup explicit import candidate fingerprint mismatch")]
    CandidateFingerprintMismatch,
    #[error("backup explicit import candidate evidence digest mismatch")]
    CandidateEvidenceMismatch,
    #[error("backup explicit import candidate fingerprint is invalid")]
    InvalidCandidateFingerprint(#[from] BackupRestoreError),
    #[error("backup explicit import plaintext evidence repo id does not match candidate repo id")]
    RepoIdMismatch,
    #[error("backup explicit import target local repo was not found for repo id {0}")]
    TargetRepoNotFound(RepoId),
    #[error("backup explicit import target local repo must be empty")]
    TargetRepoNotEmpty,
    #[error("backup explicit import candidate contains no ledger entries")]
    EmptyCandidate,
    #[error("backup explicit import ledger evidence has duplicate backup sequence {0}")]
    DuplicateBackupSeq(u64),
    #[error("backup explicit import ledger evidence has a sequence gap before {0}")]
    BackupSeqGap(u64),
    #[error(
        "backup explicit import currently requires a complete ledger image starting at backup sequence 1"
    )]
    IncompleteLedgerImage,
    #[error("backup explicit import cannot restore snapshot/blob references yet")]
    UnsupportedSnapshotOrBlobRefs,
    #[error("backup explicit import ledger evidence contains invalid ledger entry")]
    InvalidLedgerEntry(#[from] BackupPackPlaintextError),
    #[error("backup explicit import authority storage failed: {0}")]
    AuthorityStorage(String),
}

impl RepoManager {
    pub fn import_verified_restore_candidate_to_empty_local_repo(
        &self,
        input: BackupRestoreImportInput<'_>,
    ) -> Result<BackupRestoreImportReport, BackupRestoreImportError> {
        validate_import_input(input)?;
        let repo_name = self
            .find_local_repo_name_by_id(input.candidate.repo_id)
            .map_err(to_import_error)?
            .ok_or(BackupRestoreImportError::TargetRepoNotFound(
                input.candidate.repo_id,
            ))?;
        let decoded = decoded_candidate_ledger_entries(input.plaintext_packs)?;
        let imported = u64::try_from(decoded.len()).unwrap_or(u64::MAX);
        if imported == 0 {
            return Err(BackupRestoreImportError::EmptyCandidate);
        }

        let repo_scope = ops::local_repo_scope(&repo_name);
        let (first_imported_seq, last_imported_seq) = self
            .run_on_local_repo(&repo_name, |db| {
                let write_txn = db.begin_write()?;
                ensure_target_empty(&write_txn)?;
                let mut first = None;
                let mut last = 0u64;
                for (_, entry) in &decoded {
                    let seq = ops::append_op_to_txn(&write_txn, entry, &repo_scope)?;
                    if first.is_none() {
                        first = Some(seq);
                    }
                    last = seq;
                }
                write_txn.commit()?;
                Ok((first.unwrap_or(0), last))
            })
            .map_err(to_import_error)?;

        Ok(BackupRestoreImportReport {
            repo_id: input.candidate.repo_id,
            repo_name,
            candidate_fingerprint: input.candidate.fingerprint.clone(),
            imported_ledger_entries: imported,
            first_imported_seq,
            last_imported_seq,
        })
    }
}

fn validate_import_input(
    input: BackupRestoreImportInput<'_>,
) -> Result<(), BackupRestoreImportError> {
    if !input.write_gate_confirmed {
        return Err(BackupRestoreImportError::WriteGateRequired);
    }
    if input.candidate.state != RestoreAdmissionState::ExplicitImport {
        return Err(BackupRestoreImportError::CandidateNotExplicitImport);
    }
    if !input
        .candidate
        .fingerprint
        .same_sha256(input.expected_candidate_fingerprint)
    {
        return Err(BackupRestoreImportError::CandidateFingerprintMismatch);
    }
    if input.plaintext_packs.repo_id() != input.candidate.repo_id {
        return Err(BackupRestoreImportError::RepoIdMismatch);
    }
    verify_restore_candidate_fingerprint(input.candidate)?;
    let evidence_digest = plaintext_evidence_digest(input.plaintext_packs)?;
    if !evidence_digest.same_sha256(&input.candidate.plaintext_evidence_digest) {
        return Err(BackupRestoreImportError::CandidateEvidenceMismatch);
    }
    Ok(())
}

fn decoded_candidate_ledger_entries(
    plaintext_packs: &BackupPlaintextPacksResult,
) -> Result<Vec<(u64, crate::models::LedgerEntry)>, BackupRestoreImportError> {
    let mut entries = Vec::new();
    for pack in plaintext_packs.plaintext_packs() {
        if !pack.plaintext().snapshot_refs.is_empty() || !pack.plaintext().blob_refs.is_empty() {
            return Err(BackupRestoreImportError::UnsupportedSnapshotOrBlobRefs);
        }
        entries.extend(pack.plaintext().decoded_ledger_entries()?);
    }
    entries.sort_by_key(|(seq, _)| *seq);
    validate_backup_sequences(&entries)?;
    Ok(entries)
}

fn validate_backup_sequences(
    entries: &[(u64, crate::models::LedgerEntry)],
) -> Result<(), BackupRestoreImportError> {
    let Some((first, _)) = entries.first() else {
        return Ok(());
    };
    if *first != 1 {
        return Err(BackupRestoreImportError::IncompleteLedgerImage);
    }
    let mut expected = *first;
    let mut previous = None;
    for (seq, _) in entries {
        if previous == Some(*seq) {
            return Err(BackupRestoreImportError::DuplicateBackupSeq(*seq));
        }
        if *seq != expected {
            return Err(BackupRestoreImportError::BackupSeqGap(*seq));
        }
        previous = Some(*seq);
        expected = expected.saturating_add(1);
    }
    Ok(())
}

fn ensure_target_empty(write_txn: &redb::WriteTransaction) -> Result<(), BackupRestoreImportError> {
    let ledger = write_txn
        .open_table(LEDGER_OPS)
        .map_err(|err| BackupRestoreImportError::AuthorityStorage(err.to_string()))?;
    if ledger
        .last()
        .map_err(|err| BackupRestoreImportError::AuthorityStorage(err.to_string()))?
        .is_some()
    {
        return Err(BackupRestoreImportError::TargetRepoNotEmpty);
    }
    Ok(())
}

fn to_import_error(err: anyhow::Error) -> BackupRestoreImportError {
    if matches!(
        err.downcast_ref::<BackupRestoreImportError>(),
        Some(BackupRestoreImportError::TargetRepoNotEmpty)
    ) {
        BackupRestoreImportError::TargetRepoNotEmpty
    } else {
        BackupRestoreImportError::AuthorityStorage(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::restore::tests::verified_restore_evidence;
    use crate::backup::{
        BackupPlaintextPacksInput, verify_decrypted_backup_packs, verify_downloaded_backup_packs,
        verify_plaintext_backup_packs,
    };
    use crate::backup::{
        RestoreAdmissionMode, RestoreCandidateFromVerifiedPacksInput,
        admit_verified_restore_candidate,
    };
    use crate::ledger::RepoManager;
    use crate::ledger::init::RepoInitOptions;
    use crate::ledger::range;
    use tempfile::tempdir;

    #[test]
    fn imports_explicit_candidate_to_empty_local_repo() -> anyhow::Result<()> {
        let evidence = verified_restore_evidence();
        let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
            expected_repo_id: evidence.plaintext_packs.repo_id(),
            manifest_verification: &evidence.manifest_verification,
            plaintext_packs: &evidence.plaintext_packs,
            admission_mode: RestoreAdmissionMode::ExplicitImport,
            write_gate_confirmed: true,
        })?;
        let dir = tempdir()?;
        let repo = RepoManager::init_with_options(
            dir.path(),
            8,
            Some("restored"),
            RepoInitOptions {
                repo_id: Some(candidate.repo_id),
                repo_url: Some("urn:test:restore".into()),
            },
        )?;

        let report =
            repo.import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &evidence.plaintext_packs,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: true,
            })?;

        assert_eq!(report.repo_id, candidate.repo_id);
        assert_eq!(report.repo_name, "restored");
        assert_eq!(report.imported_ledger_entries, 2);
        assert_eq!(report.first_imported_seq, 1);
        assert_eq!(report.last_imported_seq, 2);
        let max_seq = repo.run_on_local_repo("restored", range::get_max_seq)?;
        assert_eq!(max_seq, 2);
        Ok(())
    }

    #[test]
    fn rejects_import_without_gate_or_explicit_mode() -> anyhow::Result<()> {
        let evidence = verified_restore_evidence();
        let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
            expected_repo_id: evidence.plaintext_packs.repo_id(),
            manifest_verification: &evidence.manifest_verification,
            plaintext_packs: &evidence.plaintext_packs,
            admission_mode: RestoreAdmissionMode::RemoteReadonly,
            write_gate_confirmed: false,
        })?;
        let dir = tempdir()?;
        let repo = RepoManager::init_with_options(
            dir.path(),
            8,
            Some("restored"),
            RepoInitOptions {
                repo_id: Some(candidate.repo_id),
                repo_url: Some("urn:test:restore".into()),
            },
        )?;

        let no_gate = repo
            .import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &evidence.plaintext_packs,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: false,
            })
            .expect_err("write gate required");
        assert!(matches!(
            no_gate,
            BackupRestoreImportError::WriteGateRequired
        ));

        let wrong_mode = repo
            .import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &evidence.plaintext_packs,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: true,
            })
            .expect_err("remote readonly candidate rejected");
        assert!(matches!(
            wrong_mode,
            BackupRestoreImportError::CandidateNotExplicitImport
        ));
        Ok(())
    }

    #[test]
    fn rejects_non_empty_target_repo() -> anyhow::Result<()> {
        let evidence = verified_restore_evidence();
        let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
            expected_repo_id: evidence.plaintext_packs.repo_id(),
            manifest_verification: &evidence.manifest_verification,
            plaintext_packs: &evidence.plaintext_packs,
            admission_mode: RestoreAdmissionMode::ExplicitImport,
            write_gate_confirmed: true,
        })?;
        let dir = tempdir()?;
        let repo = RepoManager::init_with_options(
            dir.path(),
            8,
            Some("restored"),
            RepoInitOptions {
                repo_id: Some(candidate.repo_id),
                repo_url: Some("urn:test:restore".into()),
            },
        )?;
        let first_entry = evidence.plaintext_packs.plaintext_packs()[0]
            .plaintext()
            .decoded_ledger_entries()?
            .remove(0)
            .1;
        repo.authority_storage_runtime()
            .append_local_op_in_local_repo("restored", &first_entry)?;

        let err = repo
            .import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &evidence.plaintext_packs,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: true,
            })
            .expect_err("non-empty target rejected");

        assert!(matches!(err, BackupRestoreImportError::TargetRepoNotEmpty));
        Ok(())
    }

    #[test]
    fn rejects_candidate_plaintext_evidence_mismatch() -> anyhow::Result<()> {
        let evidence = verified_restore_evidence();
        let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
            expected_repo_id: evidence.plaintext_packs.repo_id(),
            manifest_verification: &evidence.manifest_verification,
            plaintext_packs: &evidence.plaintext_packs,
            admission_mode: RestoreAdmissionMode::ExplicitImport,
            write_gate_confirmed: true,
        })?;
        let mismatched = plaintext_evidence_from_single_pack(1)?;
        let dir = tempdir()?;
        let repo = RepoManager::init_with_options(
            dir.path(),
            8,
            Some("restored"),
            RepoInitOptions {
                repo_id: Some(candidate.repo_id),
                repo_url: Some("urn:test:restore".into()),
            },
        )?;

        let err = repo
            .import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &mismatched,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: true,
            })
            .expect_err("mismatched plaintext evidence rejected");

        assert!(matches!(
            err,
            BackupRestoreImportError::CandidateEvidenceMismatch
        ));
        Ok(())
    }

    #[test]
    fn rejects_incomplete_ledger_slice_not_starting_at_one() -> anyhow::Result<()> {
        let evidence = plaintext_evidence_from_single_pack(2)?;
        let manifest_verification =
            crate::backup::restore::tests::manifest_verification_with_sequences(
                evidence
                    .pack_refs()
                    .iter()
                    .map(|pack| (pack.pack_sequence(), pack.digest().clone()))
                    .collect(),
            );
        let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
            expected_repo_id: evidence.repo_id(),
            manifest_verification: &manifest_verification,
            plaintext_packs: &evidence,
            admission_mode: RestoreAdmissionMode::ExplicitImport,
            write_gate_confirmed: true,
        })?;
        let dir = tempdir()?;
        let repo = RepoManager::init_with_options(
            dir.path(),
            8,
            Some("restored"),
            RepoInitOptions {
                repo_id: Some(candidate.repo_id),
                repo_url: Some("urn:test:restore".into()),
            },
        )?;

        let err = repo
            .import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &evidence,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: true,
            })
            .expect_err("incremental slice rejected");

        assert!(matches!(
            err,
            BackupRestoreImportError::IncompleteLedgerImage
        ));
        Ok(())
    }

    #[test]
    fn rejects_snapshot_or_blob_refs_until_restored() -> anyhow::Result<()> {
        let evidence = crate::backup::restore::tests::verified_restore_evidence_with_refs();
        let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
            expected_repo_id: evidence.plaintext_packs.repo_id(),
            manifest_verification: &evidence.manifest_verification,
            plaintext_packs: &evidence.plaintext_packs,
            admission_mode: RestoreAdmissionMode::ExplicitImport,
            write_gate_confirmed: true,
        })?;
        let dir = tempdir()?;
        let repo = RepoManager::init_with_options(
            dir.path(),
            8,
            Some("restored"),
            RepoInitOptions {
                repo_id: Some(candidate.repo_id),
                repo_url: Some("urn:test:restore".into()),
            },
        )?;

        let err = repo
            .import_verified_restore_candidate_to_empty_local_repo(BackupRestoreImportInput {
                candidate: &candidate,
                plaintext_packs: &evidence.plaintext_packs,
                expected_candidate_fingerprint: &candidate.fingerprint,
                write_gate_confirmed: true,
            })
            .expect_err("snapshot/blob refs rejected");

        assert!(matches!(
            err,
            BackupRestoreImportError::UnsupportedSnapshotOrBlobRefs
        ));
        Ok(())
    }

    fn plaintext_evidence_from_single_pack(
        pack_sequence: u64,
    ) -> anyhow::Result<crate::backup::BackupPlaintextPacksResult> {
        let fixture = crate::backup::restore::tests::single_pack_fixture(pack_sequence);
        let manifest = crate::backup::restore::tests::branch_manifest_for_single_pack(&fixture);
        let downloaded =
            verify_downloaded_backup_packs(crate::backup::BackupDownloadedPacksInput {
                branch_manifest: &manifest,
                verified_packs: vec![fixture.download_result],
            })?;
        let decrypted = verify_decrypted_backup_packs(crate::backup::BackupDecryptedPacksInput {
            downloaded_packs: &downloaded,
            opened_packs: vec![fixture.open_result],
        })?;
        Ok(verify_plaintext_backup_packs(BackupPlaintextPacksInput {
            branch_manifest: &manifest,
            decrypted_packs: &decrypted,
        })?)
    }
}
