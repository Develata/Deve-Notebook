//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 17_tech_stack#git-ecosystem-bridge
//!
//! Host-side resolution for the one executable path the Desktop sidecar may
//! inherit. The child process intentionally receives no host PATH/PATHEXT.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use deve_core::git_bridge::DEVE_GIT_EXECUTABLE_ENV;

pub(super) enum TrustedGitExecutable {
    Bound(PathBuf),
    Unavailable,
}

pub(super) fn resolve_trusted_git_executable_from_env() -> TrustedGitExecutable {
    resolve_trusted_git_executable(
        std::env::var_os(DEVE_GIT_EXECUTABLE_ENV).as_deref(),
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
        cfg!(windows),
    )
}

fn resolve_trusted_git_executable(
    explicit: Option<&OsStr>,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
    windows: bool,
) -> TrustedGitExecutable {
    if let Some(explicit) = explicit {
        return canonical_regular_file(Path::new(explicit))
            .map(TrustedGitExecutable::Bound)
            .unwrap_or(TrustedGitExecutable::Unavailable);
    }

    let candidate_names = git_candidate_names(pathext, windows);
    for directory in path.into_iter().flat_map(std::env::split_paths) {
        if !directory.is_absolute() {
            continue;
        }
        for name in &candidate_names {
            if let Some(candidate) = canonical_regular_file(&directory.join(name)) {
                return TrustedGitExecutable::Bound(candidate);
            }
        }
    }
    TrustedGitExecutable::Unavailable
}

fn git_candidate_names(pathext: Option<&OsStr>, windows: bool) -> Vec<String> {
    let mut names = vec!["git".to_string()];
    if !windows {
        return names;
    }

    let extensions = pathext
        .and_then(OsStr::to_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(".COM;.EXE;.BAT;.CMD");
    for raw in extensions.split(';') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let extension = if raw.starts_with('.') {
            raw.to_string()
        } else {
            format!(".{raw}")
        };
        if !extension[1..].chars().all(|ch| ch.is_ascii_alphanumeric()) {
            continue;
        }
        let candidate = format!("git{extension}");
        if !names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&candidate))
        {
            names.push(candidate);
        }
    }
    names
}

fn canonical_regular_file(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let canonical = std::fs::canonicalize(path).ok()?;
    canonical.metadata().ok()?.is_file().then_some(canonical)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("deve-desktop-git-resolver-{nanos}-{id}"));
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, b"test executable").expect("write test executable");
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn explicit_regular_file_is_canonicalized_and_wins() {
        let explicit_dir = TestDir::new();
        let fallback_dir = TestDir::new();
        let explicit = explicit_dir.file("chosen-git");
        fallback_dir.file("git");

        let resolved = resolve_trusted_git_executable(
            Some(explicit.as_os_str()),
            Some(fallback_dir.0.as_os_str()),
            None,
            false,
        );

        let TrustedGitExecutable::Bound(path) = resolved else {
            panic!("explicit regular file must bind");
        };
        assert_eq!(
            path,
            fs::canonicalize(explicit).expect("canonical explicit")
        );
    }

    #[test]
    fn invalid_explicit_path_does_not_fall_back_to_path() {
        let explicit_dir = TestDir::new();
        let fallback_dir = TestDir::new();
        fallback_dir.file("git");

        let resolved = resolve_trusted_git_executable(
            Some(explicit_dir.0.join("missing").as_os_str()),
            Some(fallback_dir.0.as_os_str()),
            None,
            false,
        );

        assert!(matches!(resolved, TrustedGitExecutable::Unavailable));
    }

    #[test]
    fn absolute_path_and_pathext_resolve_regular_file() {
        let directory = TestDir::new();
        let git = directory.file("git.EXE");

        let resolved = resolve_trusted_git_executable(
            None,
            Some(directory.0.as_os_str()),
            Some(OsStr::new("EXE;.CMD")),
            true,
        );

        let TrustedGitExecutable::Bound(path) = resolved else {
            panic!("PATH Git must bind");
        };
        assert_eq!(path, fs::canonicalize(git).expect("canonical Git"));
    }

    #[test]
    fn relative_path_entries_and_missing_git_are_ignored() {
        let relative = OsString::from("relative-bin");

        assert!(matches!(
            resolve_trusted_git_executable(None, Some(&relative), None, false),
            TrustedGitExecutable::Unavailable
        ));
        assert!(matches!(
            resolve_trusted_git_executable(None, None, None, false),
            TrustedGitExecutable::Unavailable
        ));
    }
}
