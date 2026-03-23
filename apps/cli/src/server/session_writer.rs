use deve_core::models::{PeerId, RepoId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriterIdentity {
    pub peer_id: PeerId,
    pub repo_id: RepoId,
}
