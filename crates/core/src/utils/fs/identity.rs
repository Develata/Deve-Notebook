//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#local-repo-removal-contract
//!
//! Durable, host-local path identity captured without following the target.

use serde::{Deserialize, Serialize};
use std::fs::File;
#[cfg(any(unix, windows))]
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "platform", rename_all = "snake_case")]
pub enum HostFileIdentity {
    Unix { device: u64, inode: u64 },
    Windows { volume: u32, file_index: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPathKind {
    RegularFile,
    Directory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostPathState {
    Exact,
    Missing,
    Changed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPathIdentity {
    path: PathBuf,
    parent_identity: HostFileIdentity,
    object_identity: HostFileIdentity,
    kind: HostPathKind,
}

impl HostPathIdentity {
    pub fn capture(path: &Path, kind: HostPathKind) -> std::io::Result<Self> {
        if !path.is_absolute() {
            return Err(invalid("identity target must be an absolute path"));
        }
        let parent = path
            .parent()
            .ok_or_else(|| invalid("identity target has no parent directory"))?;
        let parent_identity = identity_for(parent, HostPathKind::Directory)?;
        let object_identity = identity_for(path, kind)?;
        let observed_again = identity_for(path, kind)?;
        if object_identity != observed_again {
            return Err(invalid("identity target changed while it was captured"));
        }
        Ok(Self {
            path: path.to_path_buf(),
            parent_identity,
            object_identity,
            kind,
        })
    }

    pub fn revalidate(&self) -> std::io::Result<bool> {
        Ok(self.classify()? == HostPathState::Exact)
    }

    /// Classifies one originally captured pathname without treating an exact
    /// absence as an identity mismatch. Destructive owners use this to make
    /// cleanup idempotent while still rejecting parent or target replacement.
    pub fn classify(&self) -> std::io::Result<HostPathState> {
        let Some(parent) = self.path.parent() else {
            return Ok(HostPathState::Changed);
        };
        let parent_identity = match identity_for(parent, HostPathKind::Directory) {
            Ok(identity) => identity,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(HostPathState::Changed);
            }
            Err(error) => return Err(error),
        };
        if parent_identity != self.parent_identity {
            return Ok(HostPathState::Changed);
        }
        match identity_for(&self.path, self.kind) {
            Ok(identity) if identity == self.object_identity => Ok(HostPathState::Exact),
            Ok(_) => Ok(HostPathState::Changed),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(HostPathState::Missing)
            }
            Err(error) => Err(error),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn kind(&self) -> HostPathKind {
        self.kind
    }

    pub(crate) const fn parent_identity(&self) -> HostFileIdentity {
        self.parent_identity
    }

    pub(crate) const fn object_identity(&self) -> HostFileIdentity {
        self.object_identity
    }

    /// Revalidates an already-open handle against both the captured object and
    /// the captured parent/path lineage. This closes the A-to-B pathname swap
    /// that a plain "handle matches current path" check cannot detect.
    pub(crate) fn matches_open_file(&self, file: &File) -> std::io::Result<bool> {
        Ok(identity_from_handle(file)? == self.object_identity
            && self.classify()? == HostPathState::Exact)
    }
}

pub(super) fn identity_for(path: &Path, kind: HostPathKind) -> std::io::Result<HostFileIdentity> {
    let file = open_no_follow(path, kind)?;
    identity_from_handle(&file)
}

#[cfg(unix)]
pub(super) fn open_no_follow(path: &Path, kind: HostPathKind) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut flags = libc::O_NOFOLLOW | libc::O_CLOEXEC;
    if kind == HostPathKind::Directory {
        flags |= libc::O_DIRECTORY;
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)?;
    validate_kind(&file, kind)?;
    Ok(file)
}

#[cfg(windows)]
pub(super) fn open_no_follow(path: &Path, kind: HostPathKind) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let file = OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    validate_kind(&file, kind)?;
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn open_no_follow(_path: &Path, _kind: HostPathKind) -> std::io::Result<File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "host path identity is unsupported on this platform",
    ))
}

#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
fn validate_kind(file: &File, expected: HostPathKind) -> std::io::Result<()> {
    let metadata = file.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(invalid("identity target is a reparse point"));
        }
    }
    let actual = if metadata.is_file() {
        HostPathKind::RegularFile
    } else if metadata.is_dir() {
        HostPathKind::Directory
    } else {
        return Err(invalid(
            "identity target is neither a regular file nor directory",
        ));
    };
    if actual != expected {
        return Err(invalid(
            "identity target kind does not match its manifest class",
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn identity_from_handle(file: &File) -> std::io::Result<HostFileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(HostFileIdentity::Unix {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(windows)]
pub(super) fn identity_from_handle(file: &File) -> std::io::Result<HostFileIdentity> {
    let (volume, file_index) = super::windows_file_identity(file)?;
    Ok(HostFileIdentity::Windows { volume, file_index })
}

#[cfg(not(any(unix, windows)))]
pub(super) fn identity_from_handle(_file: &File) -> std::io::Result<HostFileIdentity> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "host file identity is unsupported on this platform",
    ))
}

fn invalid(detail: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, detail)
}

#[cfg(test)]
mod tests {
    use super::{HostPathIdentity, HostPathKind, HostPathState};

    #[test]
    fn identity_detects_regular_file_replacement() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = std::fs::canonicalize(dir.path())?.join("authority.redb");
        std::fs::write(&path, b"first")?;
        let identity = HostPathIdentity::capture(&path, HostPathKind::RegularFile)?;
        assert!(identity.revalidate()?);
        std::fs::remove_file(&path)?;
        std::fs::write(&path, b"second")?;
        assert!(!identity.revalidate()?);
        Ok(())
    }

    #[test]
    fn identity_rejects_kind_mismatch() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = std::fs::canonicalize(dir.path())?;
        assert!(HostPathIdentity::capture(&root, HostPathKind::RegularFile).is_err());
        Ok(())
    }

    #[test]
    fn identity_distinguishes_exact_absence_from_replacement() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = std::fs::canonicalize(dir.path())?.join("authority.redb");
        std::fs::write(&path, b"first")?;
        let identity = HostPathIdentity::capture(&path, HostPathKind::RegularFile)?;

        std::fs::remove_file(&path)?;
        assert_eq!(identity.classify()?, HostPathState::Missing);

        std::fs::write(&path, b"replacement")?;
        assert_eq!(identity.classify()?, HostPathState::Changed);
        Ok(())
    }
}
