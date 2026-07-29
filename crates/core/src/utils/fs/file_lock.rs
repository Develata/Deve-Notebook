//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!
//! Portable exclusive file-lock operations for authority lock files.
//!
//! `std::fs::File::{lock, try_lock, unlock}` are compiled out on
//! `target_os = "android"` (std's unix file-lock cfg list omits it) and return
//! `Unsupported` unconditionally, which fail-closes the mobile embedded
//! authority runtime at bootstrap. Bionic and the kernel support `flock(2)`
//! exactly as desktop Linux does — std itself locks via `flock` on Linux — so
//! every authority-lock call site routes through this module: std on
//! non-Android targets, direct `libc::flock` on Android.
//!
//! Platform semantics: on unix targets both paths take the same advisory
//! `flock` lock, scoped to the open file description and released on close.
//! On Windows std maps to `LockFileEx` (per-handle, mandatory); the call
//! sites hold exactly one guard handle and release on drop/close, a pattern
//! that behaves identically under either model. Both paths retry
//! `Interrupted` — an intentional strengthening over std, which surfaces an
//! EINTR from a blocking `flock` as an error. The Android branch cannot be
//! unit-tested on the host; the Android emulator lifecycle smoke is its
//! integration proof.

use std::fs::File;

/// Failure of a non-blocking exclusive lock attempt.
#[derive(Debug)]
pub enum FileTryLockError {
    /// Another handle holds the lock; the caller decides busy semantics.
    WouldBlock,
    /// The lock operation itself failed.
    Error(std::io::Error),
}

impl std::fmt::Display for FileTryLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WouldBlock => write!(f, "file lock is held by another handle"),
            Self::Error(error) => write!(f, "file lock failed: {error}"),
        }
    }
}

impl std::error::Error for FileTryLockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WouldBlock => None,
            Self::Error(error) => Some(error),
        }
    }
}

/// Acquires an exclusive lock, blocking until it is available.
#[cfg(not(target_os = "android"))]
pub fn lock_file_exclusive(file: &File) -> std::io::Result<()> {
    loop {
        match file.lock() {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// Attempts an exclusive lock without blocking.
#[cfg(not(target_os = "android"))]
pub fn try_lock_file_exclusive(file: &File) -> Result<(), FileTryLockError> {
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => return Err(FileTryLockError::WouldBlock),
            Err(std::fs::TryLockError::Error(error))
                if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(FileTryLockError::Error(error));
            }
        }
    }
}

/// Releases a lock previously taken on this open file description.
#[cfg(not(target_os = "android"))]
pub fn unlock_file(file: &File) -> std::io::Result<()> {
    file.unlock()
}

/// Acquires an exclusive lock, blocking until it is available.
#[cfg(target_os = "android")]
pub fn lock_file_exclusive(file: &File) -> std::io::Result<()> {
    flock(file, libc::LOCK_EX)
}

/// Attempts an exclusive lock without blocking.
#[cfg(target_os = "android")]
pub fn try_lock_file_exclusive(file: &File) -> Result<(), FileTryLockError> {
    match flock(file, libc::LOCK_EX | libc::LOCK_NB) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            Err(FileTryLockError::WouldBlock)
        }
        Err(error) => Err(FileTryLockError::Error(error)),
    }
}

/// Releases a lock previously taken on this open file description.
#[cfg(target_os = "android")]
pub fn unlock_file(file: &File) -> std::io::Result<()> {
    flock(file, libc::LOCK_UN)
}

#[cfg(target_os = "android")]
fn flock(file: &File, operation: libc::c_int) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    loop {
        // SAFETY: the fd is owned by the borrowed `File`, so it stays valid
        // for the duration of the call; flock does not touch memory.
        let result = unsafe { libc::flock(file.as_raw_fd(), operation) };
        if result == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::path::Path;

    fn open_handle(dir: &Path) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("authority.lock"))
            .expect("open lock handle")
    }

    #[test]
    fn second_handle_sees_would_block_until_first_unlocks() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let first = open_handle(dir.path());
        let second = open_handle(dir.path());

        try_lock_file_exclusive(&first).expect("first handle locks");
        match try_lock_file_exclusive(&second) {
            Err(FileTryLockError::WouldBlock) => {}
            other => panic!("expected WouldBlock for contended lock, got {other:?}"),
        }

        unlock_file(&first).expect("first handle unlocks");
        try_lock_file_exclusive(&second).expect("second handle locks after release");
        unlock_file(&second).expect("second handle unlocks");
    }

    #[test]
    fn blocking_lock_succeeds_on_uncontended_file() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let handle = open_handle(dir.path());
        lock_file_exclusive(&handle).expect("uncontended blocking lock");
        unlock_file(&handle).expect("unlock");
    }
}
