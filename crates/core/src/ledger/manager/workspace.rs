//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/index#internal-path-normalization
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/watcher#watcher-contract
//!
use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::utils::path::{to_forward_slash, validate_projection_repo_child_path};
use anyhow::{Context, Result, anyhow};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

impl RepoManager {
    /// 返回指定本地 repo 的 Projection Workspace 根目录：
    /// `<projection_base>/<safe_repo_name>--<repo_id>/`
    pub fn local_repo_workspace_root(&self, repo_name: &str) -> Result<PathBuf> {
        let info = self
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
        let locator = self.validated_projection_locator_for_repo_id(info.uuid)?;
        let segment = crate::ledger::manager::projection_locator::repo_workspace_segment(
            &info.name, info.uuid,
        )?;
        let root = locator.projection_base_abs.join(&segment);
        validate_projection_workspace_root(&locator.projection_base_abs, &root)?;
        let legacy_segment =
            crate::ledger::manager::projection_locator::safe_repo_path_segment(&info.name)?;
        let legacy_root = locator.projection_base_abs.join(legacy_segment);
        if legacy_root != root
            && legacy_root.try_exists().with_context(|| {
                format!("Failed to stat legacy Projection workspace root: {legacy_root:?}")
            })?
            && !root
                .try_exists()
                .with_context(|| format!("Failed to stat Projection workspace root: {root:?}"))?
        {
            return Err(anyhow!(
                "Projection workspace for local repo {} still uses legacy repo-name path {:?}; run local repo catalog repair to realign it to {:?}",
                info.name,
                legacy_root,
                root
            ));
        }
        Ok(root)
    }

    /// 返回指定本地 repo 下某个文档的物理路径：
    /// `<projection_base>/<safe_repo_name>--<repo_id>/<repo_path>`
    pub fn local_repo_workspace_path(&self, repo_name: &str, repo_path: &str) -> Result<PathBuf> {
        let repo_root = self.local_repo_workspace_root(repo_name)?;
        if repo_path.is_empty() {
            return Ok(repo_root);
        }
        let relative = validate_projection_repo_child_path(repo_path)
            .with_context(|| format!("Invalid Projection Workspace child path: {repo_path:?}"))?;
        let target = repo_root.join(relative);
        validate_existing_ancestor_containment(&repo_root, &target)?;
        Ok(target)
    }

    /// 返回指定本地 repo 的运行时元数据目录：
    /// `<projection_base>/<safe_repo_name>--<repo_id>/.notegit/`
    pub fn local_repo_notegit_root(&self, repo_name: &str) -> Result<PathBuf> {
        Ok(crate::utils::notegit::repo_dir(
            &self.local_repo_workspace_root(repo_name)?,
        ))
    }

    pub fn local_repo_notegit_keys_root(&self, repo_name: &str) -> Result<PathBuf> {
        Ok(crate::utils::notegit::repo_keys_dir(
            &self.local_repo_workspace_root(repo_name)?,
        ))
    }

    pub fn ensure_local_repo_workspace_identity(&self, repo_name: &str) -> Result<PathBuf> {
        let info = self
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
        let root = self.local_repo_workspace_root(repo_name)?;
        crate::utils::notegit::ensure_repo_identity_marker(&root, info.uuid, &info.name)?;
        Ok(root)
    }

    pub fn validate_local_repo_workspace_identity(&self, repo_name: &str) -> Result<PathBuf> {
        let info = self
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
        let root = self.local_repo_workspace_root(repo_name)?;
        crate::utils::notegit::validate_repo_identity_marker(&root, info.uuid)?;
        Ok(root)
    }

    /// 构造 Watcher / PersistGuard 使用的 repo-scoped key。
    pub fn local_repo_workspace_relative(&self, repo_name: &str, repo_path: &str) -> String {
        let repo_path = to_forward_slash(repo_path).trim_matches('/').to_string();
        if repo_path.is_empty() {
            repo_name.to_string()
        } else {
            format!("{repo_name}/{repo_path}")
        }
    }

    /// 将 `<repo_name>/<repo_path>` 形式的 legacy repo-scoped key 解析回本地 repo 作用域。
    ///
    /// 返回 `(repo_name, repo_id, repo_path)`；若不在本地 repo 目录下则返回 `None`。
    pub fn resolve_local_workspace_path(
        &self,
        root_relative: &str,
    ) -> Result<Option<(String, RepoId, String)>> {
        let normalized = to_forward_slash(root_relative)
            .trim_matches('/')
            .to_string();
        if normalized.is_empty() {
            return Ok(None);
        }

        let (repo_name, repo_path) = match normalized.split_once('/') {
            Some((repo_name, repo_path)) => (repo_name, repo_path),
            None => (normalized.as_str(), ""),
        };

        if repo_name.is_empty() || repo_name.starts_with('.') {
            return Ok(None);
        }

        let Some(repo_stem) = self.resolve_local_repo_stem(repo_name).map_err(|err| {
            anyhow!(
                "Broken local repo {} while resolving workspace path {}: {}",
                repo_name,
                normalized,
                err
            )
        })?
        else {
            return Ok(None);
        };
        let Some(info) = self.get_repo_info_for(None, Some(&repo_stem))? else {
            anyhow::bail!(
                "Broken local repo {} while resolving workspace path {}: repository info missing",
                repo_stem,
                normalized
            );
        };

        Ok(Some((repo_stem, info.uuid, repo_path.to_string())))
    }

    pub(crate) fn record_projection_write(&self, repo_name: &str, repo_path: &str, content: &str) {
        let relative = self.local_repo_workspace_relative(repo_name, repo_path);
        self.persist_guard.record(&relative, content);
    }

    pub(crate) fn record_projection_delete(&self, repo_name: &str, repo_path: &str) {
        let relative = self.local_repo_workspace_relative(repo_name, repo_path);
        self.persist_guard.record_delete(&relative);
    }

    pub(crate) fn clear_projection_guard(&self, repo_name: &str, repo_path: &str) {
        let relative = self.local_repo_workspace_relative(repo_name, repo_path);
        self.persist_guard.clear(&relative);
    }

    pub(crate) fn should_ignore_workspace_event(&self, repo_name: &str, repo_path: &str) -> bool {
        let Ok(repo_root) = self.local_repo_workspace_root(repo_name) else {
            return false;
        };
        let relative = self.local_repo_workspace_relative(repo_name, repo_path);
        self.persist_guard.should_ignore(&repo_root, &relative)
    }
}

fn validate_projection_workspace_root(projection_base: &Path, repo_root: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(repo_root) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| {
                format!("Failed to stat Projection Workspace root: {repo_root:?}")
            });
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(anyhow!(
            "Projection Workspace root must not be a symlink or junction: {repo_root:?}"
        ));
    }

    let canonical_base = std::fs::canonicalize(projection_base).with_context(|| {
        format!("Failed to canonicalize Projection Locator base: {projection_base:?}")
    })?;
    let canonical_root = std::fs::canonicalize(repo_root).with_context(|| {
        format!("Failed to canonicalize Projection Workspace root: {repo_root:?}")
    })?;
    if canonical_root.parent() != Some(canonical_base.as_path()) {
        return Err(anyhow!(
            "Projection Workspace root escapes canonical projection base: root={canonical_root:?}, base={canonical_base:?}"
        ));
    }
    Ok(())
}

fn validate_existing_ancestor_containment(repo_root: &Path, target: &Path) -> Result<()> {
    let canonical_root = match std::fs::symlink_metadata(repo_root) {
        Ok(_) => std::fs::canonicalize(repo_root).with_context(|| {
            format!("Failed to canonicalize Projection Workspace root: {repo_root:?}")
        })?,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| {
                format!("Failed to stat Projection Workspace root: {repo_root:?}")
            });
        }
    };

    let mut ancestor = target;
    loop {
        match std::fs::symlink_metadata(ancestor) {
            Ok(_) => {
                let canonical_ancestor = std::fs::canonicalize(ancestor).with_context(|| {
                    format!(
                        "Failed to canonicalize existing Projection Workspace ancestor: {ancestor:?}"
                    )
                })?;
                if !canonical_ancestor.starts_with(&canonical_root) {
                    return Err(anyhow!(
                        "Projection Workspace path escapes canonical root: target={target:?}, ancestor={canonical_ancestor:?}, root={canonical_root:?}"
                    ));
                }
                return Ok(());
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "Failed to stat Projection Workspace ancestor while resolving {target:?}: {ancestor:?}"
                    )
                });
            }
        }

        ancestor = ancestor.parent().ok_or_else(|| {
            anyhow!("Projection Workspace target has no existing ancestor within root: {target:?}")
        })?;
    }
}

#[cfg(test)]
mod tests;
