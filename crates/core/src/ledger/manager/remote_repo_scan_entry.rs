use crate::ledger::manager::types::RepoInfo;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct RemoteRepoEntry {
    pub path: PathBuf,
    pub stem: String,
    pub info: Option<RepoInfo>,
}

impl RemoteRepoEntry {
    pub(crate) fn is_readable(&self) -> bool {
        self.info.is_some()
    }
}

pub(super) struct RemoteRepoCatalogInfo {
    pub(super) info: RepoInfo,
    pub(super) write_back: bool,
}
