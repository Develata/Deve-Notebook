//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair
//!   - 06_backup#remote-import-state-machine

use super::artifact::{
    ArtifactEntry, CandidateArtifactEntry, RemoteImportArtifactRoot, verify_published_session,
};
use super::error::RemoteImportResult;
use super::store::{RemoteImportStore, retention::TERMINAL_RETENTION};
use super::types::{RemoteImportFailureKind, RemoteImportSessionId, RemoteImportSessionRecord};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteImportRepairFinding {
    Interrupted(RemoteImportSessionId),
    CleanupPending(RemoteImportSessionId),
    MissingArtifact(RemoteImportSessionId),
    IncompletePublication(RemoteImportSessionId),
    ArtifactTamper(RemoteImportSessionId),
    UnsafeArtifactTree(RemoteImportSessionId),
    MissingCandidate {
        session_id: RemoteImportSessionId,
        revision: u64,
    },
    ExtraCandidate {
        session_id: RemoteImportSessionId,
        revision: u64,
    },
    OrphanCandidateTemp {
        session_id: RemoteImportSessionId,
        name: String,
    },
    UnknownCandidateArtifact {
        session_id: RemoteImportSessionId,
        name: String,
    },
    ExtraBlob {
        session_id: RemoteImportSessionId,
        digest: String,
    },
    UnknownSessionArtifact {
        session_id: RemoteImportSessionId,
        name: String,
    },
    OrphanSessionArtifact(RemoteImportSessionId),
    OrphanPreparingArtifact(String),
    UnknownArtifact(String),
    RetentionDebt {
        eligible: usize,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteImportRepairReport {
    pub(crate) findings: Vec<RemoteImportRepairFinding>,
}

pub(super) fn dry_run_repair(
    store: &RemoteImportStore,
    root: Option<&RemoteImportArtifactRoot>,
) -> RemoteImportResult<RemoteImportRepairReport> {
    let records = store.list_sessions()?;
    let entries = match root {
        Some(root) => root.list_entries()?,
        None => Vec::new(),
    };
    let artifact_sessions = entries
        .iter()
        .filter_map(|entry| match entry {
            ArtifactEntry::Session(id) => Some(*id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let record_sessions = records
        .iter()
        .map(|record| record.session_id)
        .collect::<BTreeSet<_>>();
    let mut findings = Vec::new();
    for record in &records {
        inspect_record(root, record, &artifact_sessions, &mut findings);
    }
    for entry in entries {
        match entry {
            ArtifactEntry::Session(id) if !record_sessions.contains(&id) => {
                findings.push(RemoteImportRepairFinding::OrphanSessionArtifact(id));
            }
            ArtifactEntry::Preparing(name) => {
                findings.push(RemoteImportRepairFinding::OrphanPreparingArtifact(name));
            }
            ArtifactEntry::Unknown(name) => {
                findings.push(RemoteImportRepairFinding::UnknownArtifact(name));
            }
            ArtifactEntry::Session(_) => {}
        }
    }
    let eligible = records
        .iter()
        .filter(|record| record.state.is_terminal() && !record.cleanup_pending)
        .count();
    if eligible > TERMINAL_RETENTION {
        findings.push(RemoteImportRepairFinding::RetentionDebt {
            eligible,
            limit: TERMINAL_RETENTION,
        });
    }
    findings.sort_by_key(|finding| format!("{finding:?}"));
    Ok(RemoteImportRepairReport { findings })
}

fn inspect_record(
    root: Option<&RemoteImportArtifactRoot>,
    record: &RemoteImportSessionRecord,
    artifact_sessions: &BTreeSet<RemoteImportSessionId>,
    findings: &mut Vec<RemoteImportRepairFinding>,
) {
    if record.state == crate::remote_import::types::RemoteImportState::Preparing
        || record
            .failure
            .as_ref()
            .is_some_and(|failure| failure.kind == RemoteImportFailureKind::Interrupted)
    {
        findings.push(RemoteImportRepairFinding::Interrupted(record.session_id));
    }
    if record.cleanup_pending {
        findings.push(RemoteImportRepairFinding::CleanupPending(record.session_id));
    }
    let has_artifact = artifact_sessions.contains(&record.session_id);
    let requires_artifact = !record.state.is_terminal() && !record.cleanup_pending;
    if record.source_snapshot.is_some() && !has_artifact && requires_artifact {
        findings.push(RemoteImportRepairFinding::MissingArtifact(
            record.session_id,
        ));
        return;
    }
    if record.source_snapshot.is_none() && has_artifact {
        findings.push(RemoteImportRepairFinding::IncompletePublication(
            record.session_id,
        ));
        return;
    }
    if record.source_snapshot.is_some() && has_artifact {
        let Some(root) = root else {
            findings.push(RemoteImportRepairFinding::MissingArtifact(
                record.session_id,
            ));
            return;
        };
        let layout = match root.inventory_session_layout(record.session_id) {
            Ok(layout) => layout,
            Err(_) => {
                findings.push(RemoteImportRepairFinding::UnsafeArtifactTree(
                    record.session_id,
                ));
                return;
            }
        };
        for name in layout.unknown_entries {
            findings.push(RemoteImportRepairFinding::UnknownSessionArtifact {
                session_id: record.session_id,
                name,
            });
        }
        let expected_revision = record
            .candidate
            .as_ref()
            .map(|candidate| candidate.revision.get());
        match root.list_candidate_entries(record.session_id) {
            Ok(entries) => {
                let mut has_expected = false;
                for entry in entries {
                    match entry {
                        CandidateArtifactEntry::Revision(revision)
                            if Some(revision) == expected_revision =>
                        {
                            has_expected = true;
                        }
                        CandidateArtifactEntry::Revision(revision)
                            if expected_revision.is_some_and(|expected| revision < expected) => {}
                        CandidateArtifactEntry::Revision(revision) => {
                            findings.push(RemoteImportRepairFinding::ExtraCandidate {
                                session_id: record.session_id,
                                revision,
                            })
                        }
                        CandidateArtifactEntry::Preparing(name) => {
                            findings.push(RemoteImportRepairFinding::OrphanCandidateTemp {
                                session_id: record.session_id,
                                name,
                            })
                        }
                        CandidateArtifactEntry::Unknown(name) => {
                            findings.push(RemoteImportRepairFinding::UnknownCandidateArtifact {
                                session_id: record.session_id,
                                name,
                            })
                        }
                    }
                }
                if let Some(revision) = expected_revision
                    && !has_expected
                {
                    findings.push(RemoteImportRepairFinding::MissingCandidate {
                        session_id: record.session_id,
                        revision,
                    });
                    return;
                }
            }
            Err(_) => {
                findings.push(RemoteImportRepairFinding::UnsafeArtifactTree(
                    record.session_id,
                ));
                return;
            }
        }
        match verify_published_session(root, record) {
            Ok(manifest) => {
                let expected_blobs = manifest
                    .into_iter()
                    .map(|entry| entry.digest.to_hex())
                    .collect::<BTreeSet<_>>();
                for digest in layout.blob_names {
                    if !expected_blobs.contains(&digest) {
                        findings.push(RemoteImportRepairFinding::ExtraBlob {
                            session_id: record.session_id,
                            digest,
                        });
                    }
                }
            }
            Err(_) => {
                findings.push(RemoteImportRepairFinding::ArtifactTamper(record.session_id));
            }
        }
    }
}
