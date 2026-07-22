//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Manifest-bound destructive cut used only by state owners. This module is
//! crate-private so callers cannot turn it into a generic pathname deleter.

use super::identity::identity_for;
use super::{HostFileIdentity, HostPathIdentity, HostPathKind, HostPathState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::{delete_directory_pinned, delete_file_pinned, native_rename_no_replace};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::{delete_directory_pinned, delete_file_pinned, native_rename_no_replace};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostQuarantinePlan {
    original: HostPathIdentity,
    quarantine_path: PathBuf,
    quarantine_parent_identity: HostFileIdentity,
    allow_distinct_parent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostQuarantineCut {
    plan: HostQuarantinePlan,
    quarantined: HostPathIdentity,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
impl HostQuarantinePlan {
    pub(crate) fn same_parent(
        original: HostPathIdentity,
        quarantine_path: PathBuf,
    ) -> std::io::Result<Self> {
        Self::prepare(original, quarantine_path, false)
    }

    pub(crate) fn distinct_parent_same_filesystem(
        original: HostPathIdentity,
        quarantine_path: PathBuf,
    ) -> std::io::Result<Self> {
        Self::prepare(original, quarantine_path, true)
    }

    fn prepare(
        original: HostPathIdentity,
        quarantine_path: PathBuf,
        allow_distinct_parent: bool,
    ) -> std::io::Result<Self> {
        if !quarantine_path.is_absolute() || quarantine_path.file_name().is_none() {
            return Err(invalid("quarantine target must be one absolute child path"));
        }
        let original_parent = original
            .path()
            .parent()
            .ok_or_else(|| invalid("quarantine source has no parent"))?;
        let quarantine_parent = quarantine_path
            .parent()
            .ok_or_else(|| invalid("quarantine target has no parent"))?;
        if !allow_distinct_parent && original_parent != quarantine_parent {
            return Err(invalid(
                "owner-root quarantine must remain in the same parent",
            ));
        }
        if original.kind() == HostPathKind::Directory
            && quarantine_path.starts_with(original.path())
        {
            return Err(invalid("quarantine target must not be inside its source"));
        }
        if path_entry_exists(&quarantine_path)? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "quarantine target already exists",
            ));
        }
        let quarantine_parent_identity = identity_for(quarantine_parent, HostPathKind::Directory)?;
        ensure_same_filesystem(original.parent_identity(), quarantine_parent_identity)?;
        if original.classify()? != HostPathState::Exact {
            return Err(invalid("quarantine source identity is not exact"));
        }
        Ok(Self {
            original,
            quarantine_path,
            quarantine_parent_identity,
            allow_distinct_parent,
        })
    }

    pub(crate) fn cut(&self) -> std::io::Result<HostQuarantineCut> {
        match self.original.classify()? {
            HostPathState::Exact => self.cut_exact(),
            HostPathState::Missing => self.rebuild_cut(),
            HostPathState::Changed => Err(invalid("quarantine source identity changed")),
        }
    }

    /// Reconstructs a cut that completed before its owner checkpoint was
    /// persisted, without initiating a new filesystem mutation.
    pub(crate) fn observe_cut(&self) -> std::io::Result<Option<HostQuarantineCut>> {
        match self.original.classify()? {
            HostPathState::Exact => {
                self.validate_parents()?;
                if path_entry_exists(&self.quarantine_path)? {
                    return Err(invalid("original and quarantine objects are both present"));
                }
                Ok(None)
            }
            HostPathState::Missing => self.rebuild_cut().map(Some),
            HostPathState::Changed => Err(invalid("quarantine source identity changed")),
        }
    }

    pub(crate) fn revalidate_prepared(&self) -> std::io::Result<bool> {
        if self.validate_parents().is_err() || self.original.classify()? != HostPathState::Exact {
            return Ok(false);
        }
        Ok(!path_entry_exists(&self.quarantine_path)?)
    }

    pub(crate) fn is_fully_absent(&self) -> std::io::Result<bool> {
        Ok(self.original.classify()? == HostPathState::Missing
            && !path_entry_exists(&self.quarantine_path)?)
    }

    pub(crate) fn quarantine_is_absent(&self) -> std::io::Result<bool> {
        let parent = self
            .quarantine_path
            .parent()
            .ok_or_else(|| invalid("quarantine target has no parent"))?;
        Ok(
            identity_for(parent, HostPathKind::Directory)? == self.quarantine_parent_identity
                && !path_entry_exists(&self.quarantine_path)?,
        )
    }

    fn cut_exact(&self) -> std::io::Result<HostQuarantineCut> {
        self.validate_parents()?;
        if path_entry_exists(&self.quarantine_path)? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "quarantine target appeared before the cut",
            ));
        }
        native_rename_no_replace(
            &self.original,
            &self.quarantine_path,
            self.quarantine_parent_identity,
        )?;
        self.rebuild_cut()
    }

    fn rebuild_cut(&self) -> std::io::Result<HostQuarantineCut> {
        self.validate_parents()?;
        if self.original.classify()? != HostPathState::Missing {
            return Err(invalid("quarantine source is not absent after the cut"));
        }
        let quarantined = HostPathIdentity::capture(&self.quarantine_path, self.original.kind())?;
        if quarantined.parent_identity() != self.quarantine_parent_identity
            || quarantined.object_identity() != self.original.object_identity()
        {
            return Err(invalid(
                "quarantine target does not contain the manifested object",
            ));
        }
        Ok(HostQuarantineCut {
            plan: self.clone(),
            quarantined,
        })
    }

    fn validate_parents(&self) -> std::io::Result<()> {
        let original_parent = self
            .original
            .path()
            .parent()
            .ok_or_else(|| invalid("quarantine source has no parent"))?;
        let quarantine_parent = self
            .quarantine_path
            .parent()
            .ok_or_else(|| invalid("quarantine target has no parent"))?;
        if identity_for(original_parent, HostPathKind::Directory)?
            != self.original.parent_identity()
            || identity_for(quarantine_parent, HostPathKind::Directory)?
                != self.quarantine_parent_identity
        {
            return Err(invalid("quarantine parent identity changed"));
        }
        if !self.allow_distinct_parent && original_parent != quarantine_parent {
            return Err(invalid("same-parent quarantine plan changed topology"));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn original(&self) -> &HostPathIdentity {
        &self.original
    }

    #[cfg(test)]
    pub(crate) fn quarantine_path(&self) -> &Path {
        &self.quarantine_path
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
impl HostQuarantineCut {
    pub(crate) fn delete(&self) -> std::io::Result<()> {
        if !self.plan.allow_distinct_parent
            && self.plan.original.classify()? != HostPathState::Missing
        {
            return Err(invalid("original object reappeared after quarantine"));
        }
        match self.quarantined.classify()? {
            HostPathState::Missing => return Ok(()),
            HostPathState::Changed => {
                return Err(invalid(
                    "quarantine object identity changed before deletion",
                ));
            }
            HostPathState::Exact => {}
        }
        delete_pinned_identity(&self.quarantined)?;
        if self.quarantined.classify()? != HostPathState::Missing {
            return Err(invalid("quarantine object still exists after deletion"));
        }
        Ok(())
    }

    pub(crate) fn is_deleted(&self) -> std::io::Result<bool> {
        Ok(self.plan.original.classify()? == HostPathState::Missing
            && self.quarantined.classify()? == HostPathState::Missing)
    }

    pub(crate) fn is_quarantined_exact(&self) -> std::io::Result<bool> {
        Ok(self.quarantined.classify()? == HostPathState::Exact)
    }

    #[cfg(test)]
    pub(crate) fn is_exclusive_quarantine(&self) -> std::io::Result<bool> {
        let (original, quarantined) = self.exclusive_quarantine_states()?;
        Ok(original == HostPathState::Missing && quarantined == HostPathState::Exact)
    }

    pub(crate) fn original_path_is_absent(&self) -> std::io::Result<bool> {
        Ok(!path_entry_exists(self.plan.original.path())?)
    }

    pub(crate) fn exclusive_quarantine_states(
        &self,
    ) -> std::io::Result<(HostPathState, HostPathState)> {
        Ok((self.plan.original.classify()?, self.quarantined.classify()?))
    }

    pub(crate) fn path(&self) -> &Path {
        self.quarantined.path()
    }

    pub(crate) fn belongs_to(&self, plan: &HostQuarantinePlan) -> bool {
        &self.plan == plan
    }
}

pub(crate) fn delete_pinned_identity(identity: &HostPathIdentity) -> std::io::Result<()> {
    match identity.kind() {
        HostPathKind::RegularFile => delete_file_pinned(identity),
        HostPathKind::Directory => delete_directory_pinned(identity),
    }
}

fn path_entry_exists(path: &Path) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn ensure_same_filesystem(
    source_parent: HostFileIdentity,
    destination_parent: HostFileIdentity,
) -> std::io::Result<()> {
    let same = match (source_parent, destination_parent) {
        (
            HostFileIdentity::Unix { device: left, .. },
            HostFileIdentity::Unix { device: right, .. },
        ) => left == right,
        (
            HostFileIdentity::Windows { volume: left, .. },
            HostFileIdentity::Windows { volume: right, .. },
        ) => left == right,
        _ => false,
    };
    if same {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::CrossesDevices,
            "quarantine rename must stay on one filesystem",
        ))
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    windows
)))]
fn native_rename_no_replace(
    _original: &HostPathIdentity,
    _destination: &Path,
    _destination_parent_identity: HostFileIdentity,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "manifest-bound quarantine is unsupported on this platform",
    ))
}

#[cfg(not(any(unix, windows)))]
fn delete_file_pinned(_identity: &HostPathIdentity) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "scoped quarantine unlink is unsupported on this platform",
    ))
}

#[cfg(not(any(unix, windows)))]
fn delete_directory_pinned(_identity: &HostPathIdentity) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "scoped quarantine tree removal is unsupported on this platform",
    ))
}

fn invalid(detail: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_plan(root: &Path, name: &str) -> std::io::Result<HostQuarantinePlan> {
        let source = root.join(name);
        std::fs::write(&source, b"owned")?;
        HostQuarantinePlan::same_parent(
            HostPathIdentity::capture(&source, HostPathKind::RegularFile)?,
            root.join(format!(".deve-removing-{name}")),
        )
    }

    #[test]
    fn quarantine_file_cut_rebuild_and_delete_are_exact() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = std::fs::canonicalize(dir.path())?;
        let plan = file_plan(&root, "authority.redb")?;
        let cut = plan.cut()?;
        assert!(!plan.original().path().exists());
        assert!(plan.quarantine_path().exists());
        assert_eq!(plan.cut()?, cut);
        cut.delete()?;
        assert!(cut.is_deleted()?);
        cut.delete()?;
        Ok(())
    }

    #[test]
    fn quarantine_directory_delete_is_bound_to_the_moved_root() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = std::fs::canonicalize(dir.path())?;
        let source = root.join("remote-import-owner");
        std::fs::create_dir(&source)?;
        std::fs::write(source.join("payload"), b"owned")?;
        let plan = HostQuarantinePlan::same_parent(
            HostPathIdentity::capture(&source, HostPathKind::Directory)?,
            root.join(".deve-removing-remote-import-owner"),
        )?;
        let cut = plan.cut()?;
        assert!(cut.is_exclusive_quarantine()?);
        cut.delete()?;
        assert!(cut.is_deleted()?);
        Ok(())
    }

    #[test]
    fn quarantine_never_replaces_a_destination() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = std::fs::canonicalize(dir.path())?;
        let plan = file_plan(&root, "authority.redb")?;
        std::fs::write(plan.quarantine_path(), b"foreign")?;
        let error = plan
            .cut()
            .expect_err("destination collision must fail closed");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(plan.original().path())?, b"owned");
        assert_eq!(std::fs::read(plan.quarantine_path())?, b"foreign");
        Ok(())
    }

    #[test]
    fn quarantine_rejects_parent_replacement() -> std::io::Result<()> {
        let outer = tempfile::tempdir()?;
        let root = outer.path().join("owner");
        std::fs::create_dir(&root)?;
        let root = std::fs::canonicalize(root)?;
        let plan = file_plan(&root, "authority.redb")?;
        let displaced = outer.path().join("displaced");
        std::fs::rename(&root, &displaced)?;
        std::fs::create_dir(&root)?;
        let error = plan.cut().expect_err("replaced parent must fail closed");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(displaced.join("authority.redb"))?, b"owned");
        Ok(())
    }

    #[test]
    fn quarantine_rejects_a_replaced_source_identity() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = std::fs::canonicalize(dir.path())?;
        let plan = file_plan(&root, "authority.redb")?;
        std::fs::remove_file(plan.original().path())?;
        std::fs::write(plan.original().path(), b"replacement")?;
        let error = plan.cut().expect_err("source replacement must fail closed");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(plan.original().path())?, b"replacement");
        Ok(())
    }

    #[test]
    fn quarantine_both_missing_without_a_cut_is_repair_required() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = std::fs::canonicalize(dir.path())?;
        let plan = file_plan(&root, "authority.redb")?;
        std::fs::remove_file(plan.original().path())?;
        let error = plan
            .cut()
            .expect_err("unproven both-missing must fail closed");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        Ok(())
    }
}
