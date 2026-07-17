//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Copy utility regression tests.

use super::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn prepared_asset_copy_skips_markdown_and_applies_assets() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src_dir");
    let dst = tmp.path().join("dst_dir");

    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.md"), "content a").unwrap();
    fs::write(src.join("a.txt"), "asset a").unwrap();
    fs::write(src.join("sub/b.md"), "content b").unwrap();
    fs::write(src.join("sub/c.png"), "asset c").unwrap();

    let prepared = prepare_dir_asset_copies(&src, &dst).unwrap();
    apply_prepared_asset_copies(&prepared).unwrap();

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
    let dir = tmp.path().join("workspace");

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
    let dir = tmp.path().join("workspace");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("index.md"), "").unwrap();

    let err = collect_md_files(&dir, &tmp.path().join("other"))
        .expect_err("base mismatch must fail closed");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn test_collect_dirs_fails_closed_when_dir_escapes_base() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("workspace");
    fs::create_dir_all(dir.join("notes")).unwrap();

    let err = collect_dirs(&dir, &tmp.path().join("other")).expect_err("base mismatch must fail");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn relative_path_under_base_normalizes_mixed_separators() {
    let base = Path::new("workspace");
    let path = Path::new("workspace/notes\\daily.md");

    let rel = relative_path_under_base(base, path).unwrap();

    assert_eq!(rel, "notes/daily.md");
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
    let prepared = prepare_dir_asset_copies(&tmp.path().join("level_0"), &dst).unwrap();
    apply_prepared_asset_copies(&prepared).unwrap();

    let mut check_path = dst.clone();
    for i in 1..100 {
        check_path = check_path.join(format!("level_{}", i));
    }
    assert!(!check_path.join("deep.md").exists());
    assert!(check_path.join("deep.bin").exists());
}

#[test]
fn prepared_asset_copy_rejects_source_drift_before_copy() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("asset.bin"), "before").unwrap();
    let prepared = prepare_dir_asset_copies(&src, &dst).unwrap();

    fs::write(src.join("asset.bin"), "changed payload").unwrap();
    let error = apply_prepared_asset_copies(&prepared).expect_err("source drift must reject");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!dst.join("asset.bin").exists());
}

#[test]
fn prepared_asset_copy_rejects_destination_drift() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("asset.bin"), "source").unwrap();
    let prepared = prepare_dir_asset_copies(&src, &dst).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("asset.bin"), "newer").unwrap();

    let error = apply_prepared_asset_copies(&prepared).expect_err("destination drift must reject");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read_to_string(dst.join("asset.bin")).unwrap(), "newer");
}
