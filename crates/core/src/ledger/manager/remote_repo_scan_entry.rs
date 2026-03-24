use crate::ledger::manager::types::RepoInfo;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct RemoteRepoEntry {
    pub path: PathBuf,
    pub stem: String,
    pub info: Option<RepoInfo>,
}

impl RemoteRepoEntry {
    pub(crate) fn is_readable(&self) -> bool {
        self.info.is_some()
    }

    pub(crate) fn is_switchable(&self) -> bool {
        self.info.as_ref().is_some_and(|info| {
            info.url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
        })
    }
}

pub(super) struct RemoteRepoCatalogInfo {
    pub(super) info: RepoInfo,
    pub(super) write_back: bool,
}
