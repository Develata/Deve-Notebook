//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitMirrorStatusError {
    #[error("failed to inspect .notegit presence: {message}")]
    NotegitPresence { message: String },
    #[error("failed to inspect .git metadata presence: {message}")]
    GitMetadataPresence { message: String },
    #[error("failed to inspect .git metadata: {message}")]
    GitMetadataInspect { message: String },
    #[error("failed to inspect .gitignore .notegit protection: {message}")]
    GitignoreProtection { message: String },
}

impl From<GitMirrorStatusError> for String {
    fn from(err: GitMirrorStatusError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitMirrorStoreError {
    #[error("failed to initialize Git mirror store: {message}")]
    Init { message: String },
    #[error("failed to read Git mirror record {deve_commit_id}: {message}")]
    ReadRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("failed to list Git mirror records: {message}")]
    ListRecords { message: String },
    #[error("failed to decode Git mirror record {deve_commit_id}: {message}")]
    DecodeRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("failed to encode Git mirror record {deve_commit_id}: {message}")]
    EncodeRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("failed to write Git mirror record {deve_commit_id}: {message}")]
    WriteRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("Git mirror record not found for Deve commit {deve_commit_id}")]
    MissingRecord { deve_commit_id: String },
    #[error("Git mirror commit object id is invalid for Deve commit {deve_commit_id}")]
    InvalidGitCommitId { deve_commit_id: String },
}

impl From<GitMirrorStoreError> for String {
    fn from(err: GitMirrorStoreError) -> Self {
        err.to_string()
    }
}
