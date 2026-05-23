use super::*;

#[test]
fn test_capability_check_net() {
    let cap = Capability {
        allow_net: vec!["api.github.com".to_string(), "google.com".to_string()],
        ..Default::default()
    };

    assert!(cap.check_net("api.github.com"));
    assert!(!cap.check_net("evil.com"));
}

#[test]
fn test_capability_check_fs() {
    let cap = Capability {
        allow_fs_read: vec![PathBuf::from("/notes/default"), PathBuf::from("C:\\Notes")],
        allow_fs_write: vec![PathBuf::from("/notes/default/public")],
        ..Default::default()
    };

    // Read checks
    assert!(cap.check_read(Path::new("/notes/default/notes.md")));
    assert!(cap.check_read(Path::new("C:\\Notes\\file.txt")));
    assert!(!cap.check_read(Path::new("/etc/passwd")));

    // Write checks
    assert!(cap.check_write(Path::new("/notes/default/public/log.txt")));
    assert!(!cap.check_write(Path::new("/notes/default/private.md")));

    // Path Traversal check
    assert!(!cap.check_read(Path::new("/notes/default/../etc/passwd")));
    assert!(!cap.check_write(Path::new("/notes/default/public/../../private.md")));
}

#[test]
fn test_capability_does_not_match_sibling_prefixes() {
    let cap = Capability {
        allow_fs_read: vec![PathBuf::from("/notes/default"), PathBuf::from("C:\\Notes")],
        ..Default::default()
    };

    assert!(!cap.check_read(Path::new("/notes/defaults/notes.md")));
    assert!(!cap.check_read(Path::new("C:\\Notes2\\file.txt")));
}

#[test]
fn test_capability_empty_prefix_does_not_match_absolute_paths() {
    let cap = Capability {
        allow_fs_read: vec![PathBuf::from("."), PathBuf::from("")],
        allow_fs_write: vec![PathBuf::from("."), PathBuf::from("")],
        ..Default::default()
    };

    assert!(!cap.check_read(Path::new("/etc/passwd")));
    assert!(!cap.check_write(Path::new("/tmp/output.md")));
}

#[test]
fn test_capability_normalizes_mixed_windows_separators() {
    let cap = Capability {
        allow_fs_read: vec![PathBuf::from("C:\\Notes")],
        allow_fs_write: vec![PathBuf::from("C:/Notes/Public")],
        ..Default::default()
    };

    assert!(cap.check_read(Path::new("C:/Notes/sub/file.md")));
    assert!(cap.check_write(Path::new("C:\\Notes\\Public\\log.txt")));
    assert!(!cap.check_write(Path::new("C:\\Notes\\Private\\log.txt")));
}

#[test]
fn test_capability_check_env() {
    let cap = Capability {
        allow_env: vec!["GITHUB_TOKEN".to_string()],
        ..Default::default()
    };

    assert!(cap.check_env("GITHUB_TOKEN"));
    assert!(!cap.check_env("SECRET_KEY"));
}
