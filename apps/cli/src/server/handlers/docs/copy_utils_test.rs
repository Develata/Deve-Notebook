//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Copy utility regression tests.

use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_copy_dir_assets_only_skips_markdown() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src_dir");
    let dst = tmp.path().join("dst_dir");

    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.md"), "content a").unwrap();
    fs::write(src.join("a.txt"), "asset a").unwrap();
    fs::write(src.join("sub/b.md"), "content b").unwrap();
    fs::write(src.join("sub/c.png"), "asset c").unwrap();

    copy_dir_assets_only(&src, &dst).unwrap();

    assert!(!dst.join("a.md").exists());
    assert!(!dst.join("sub/b.md").exists());
    assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "asset a");
    assert_eq!(
        fs::read_to_string(dst.join("sub/c.png")).unwrap(),
        "asset c"
    );
}

#[test]
fn test_collect_md_files() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("vault");

    fs::create_dir_all(dir.join("notes")).unwrap();
    fs::write(dir.join("index.md"), "").unwrap();
    fs::write(dir.join("notes/daily.md"), "").unwrap();
    fs::write(dir.join("notes/ignore.txt"), "").unwrap();

    let files = collect_md_files(&dir, &dir).unwrap();
    assert!(files.contains(&"index.md".to_string()));
    assert!(files.contains(&"notes/daily.md".to_string()));
    assert!(!files.iter().any(|f| f.ends_with(".txt")));
}

#[test]
fn test_collect_md_files_fails_closed_when_file_escapes_base() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("vault");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("index.md"), "").unwrap();

    let err = collect_md_files(&dir, &tmp.path().join("other"))
        .expect_err("base mismatch must fail closed");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn test_collect_dirs_fails_closed_when_dir_escapes_base() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("vault");
    fs::create_dir_all(dir.join("notes")).unwrap();

    let err = collect_dirs(&dir, &tmp.path().join("other")).expect_err("base mismatch must fail");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn test_deep_directory_no_stack_overflow() {
    let tmp = tempdir().unwrap();
    let mut current = tmp.path().to_path_buf();

    for i in 0..100 {
        current = current.join(format!("level_{}", i));
    }
    fs::create_dir_all(&current).unwrap();
    fs::write(current.join("deep.md"), "deep content").unwrap();
    fs::write(current.join("deep.bin"), "deep asset").unwrap();

    let dst = tmp.path().join("dst");
    copy_dir_assets_only(&tmp.path().join("level_0"), &dst).unwrap();

    let mut check_path = dst.clone();
    for i in 1..100 {
        check_path = check_path.join(format!("level_{}", i));
    }
    assert!(!check_path.join("deep.md").exists());
    assert!(check_path.join("deep.bin").exists());
}
