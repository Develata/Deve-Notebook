use std::path::{Path, PathBuf};

pub fn repo_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".notegit")
}

pub fn is_internal_repo_path(path: &str) -> bool {
    crate::utils::path::to_forward_slash(path)
        .split('/')
        .any(|segment| segment == ".notegit")
}

pub fn repo_keys_dir(repo_root: &Path) -> PathBuf {
    repo_dir(repo_root).join("keys")
}

pub fn host_dir(ledger_root: &Path) -> PathBuf {
    ledger_root.join(".host")
}

pub fn host_keys_dir(ledger_root: &Path) -> PathBuf {
    host_dir(ledger_root).join("keys")
}
