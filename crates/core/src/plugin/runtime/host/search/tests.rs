#[cfg(unix)]
use super::walk::next_walk_entry;
use super::walk::read_searchable_text;
#[cfg(unix)]
use ignore::WalkBuilder;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
fn read_searchable_text_skips_invalid_utf8_files() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("binary.bin");
    std::fs::write(&path, [0xff_u8, 0xfe_u8, 0xfd_u8]).expect("write");

    let text = read_searchable_text(&path).expect("binary files are skipped");
    assert!(text.is_none());
}

#[cfg(unix)]
#[test]
fn read_searchable_text_fails_closed_on_unreadable_file() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("secret.md");
    std::fs::write(&path, "secret").expect("write");

    let original = std::fs::metadata(&path).expect("metadata").permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&path, blocked).expect("chmod");

    let err = read_searchable_text(&path).expect_err("unreadable file must fail closed");

    std::fs::set_permissions(&path, original).expect("restore perms");
    assert!(err.to_string().contains("Search read failed"));
}

#[cfg(unix)]
#[test]
fn next_walk_entry_fails_closed_on_unreadable_directory() {
    let dir = tempdir().expect("tempdir");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("mkdir");
    std::fs::write(blocked.join("note.md"), "hello").expect("write");

    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut denied = original.clone();
    denied.set_mode(0o000);
    std::fs::set_permissions(&blocked, denied).expect("chmod");

    let walker = WalkBuilder::new(dir.path())
        .hidden(true)
        .git_ignore(true)
        .build();
    let mut walk_error = None;
    for entry in walker {
        if let Err(err) = entry {
            walk_error = Some(err);
            break;
        }
    }

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    let err = next_walk_entry(
        Err(walk_error.expect("unreadable dir should produce walk error")),
        dir.path(),
        "Search walk",
    )
    .expect_err("walk error must fail closed");
    assert!(err.to_string().contains("Search walk failed under"));
}
