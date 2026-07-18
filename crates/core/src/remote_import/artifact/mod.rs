//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 06_backup#remote-import-resource-contract
//!   - 06_backup#projection-backup-secret-ref-contract

mod capture;
mod durability;
mod root;
mod verify;

pub(super) use capture::{ArtifactCapture, validate_remote_path};
pub(super) use root::{ArtifactEntry, CandidateArtifactEntry, RemoteImportArtifactRoot};
pub(super) use verify::{
    publish_candidate_revision, verify_exact_published_session, verify_published_session,
};

pub(super) const MAX_FILE_COUNT: usize = 2_048;
pub(super) const MAX_FILE_PAYLOAD_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_TOTAL_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const MAX_PATH_BYTES: usize = 1_024;
pub(super) const MAX_TOTAL_PATH_BYTES: usize = 2 * 1024 * 1024;

pub(super) const MANIFEST_FILE: &str = "source-manifest.json";
pub(super) const CANDIDATES_DIR: &str = "candidates";
pub(super) const BLOBS_DIR: &str = "blobs";

pub(super) fn candidate_file(revision: u64) -> String {
    format!("{revision}.json")
}
