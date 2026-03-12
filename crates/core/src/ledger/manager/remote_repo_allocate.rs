use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::Result;
use std::path::PathBuf;

impl RepoManager {
    pub(crate) fn allocate_remote_repo_path(
        &self,
        peer_id: &PeerId,
        info: &RepoInfo,
    ) -> Result<PathBuf> {
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());
        std::fs::create_dir_all(&peer_dir)?;
        let base = normalized_repo_stem(&info.name, &info.uuid.to_string());
        for suffix in 0.. {
            let stem = if suffix == 0 {
                base.clone()
            } else {
                format!("{}-{}", base, suffix)
            };
            let path = peer_dir.join(format!("{}.redb", stem));
            if !path.exists() {
                return Ok(path);
            }
            if let Some(current) = Self::read_repo_info_from_path(&path)?
                && current.uuid == info.uuid
            {
                if stem_matches_base(&stem, &base) {
                    return Ok(path);
                }
                continue;
            }
        }
        unreachable!("remote repo path allocator must terminate")
    }
}

fn normalized_repo_stem(name: &str, fallback: &str) -> String {
    let trimmed = name.trim_end_matches(".redb").trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    trimmed.replace(['/', '\\'], "_")
}

fn stem_matches_base(stem: &str, base: &str) -> bool {
    stem == base
        || stem
            .strip_prefix(base)
            .and_then(|suffix| suffix.strip_prefix('-'))
            .map(|suffix| suffix.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::stem_matches_base;

    #[test]
    fn collision_suffix_is_treated_as_same_selector_family() {
        assert!(stem_matches_base("wiki", "wiki"));
        assert!(stem_matches_base("wiki-1", "wiki"));
        assert!(stem_matches_base("wiki-23", "wiki"));
        assert!(!stem_matches_base("legacy", "wiki"));
        assert!(!stem_matches_base("wiki-copy", "wiki"));
    }
}
