use std::path::{Path, PathBuf};

pub fn dir(vault_root: &Path) -> PathBuf {
    vault_root.join(".notegit")
}

pub fn keys_dir(vault_root: &Path) -> PathBuf {
    dir(vault_root).join("keys")
}

pub fn mcp_config_path(vault_root: &Path) -> PathBuf {
    dir(vault_root).join("mcp.json")
}

pub fn legacy_flat_dir(vault_root: &Path) -> PathBuf {
    dir(vault_root).join("legacy-flat")
}

pub fn legacy_flat_conflicts_dir(vault_root: &Path) -> PathBuf {
    dir(vault_root).join("legacy-flat-conflicts")
}
