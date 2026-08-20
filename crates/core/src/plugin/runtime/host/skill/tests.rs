use super::{list_all_skills_in, load_skill_by_name_in};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn assert_skill_file_error(err: anyhow::Error, expected_kinds: &[std::io::ErrorKind]) {
    assert_eq!(err.to_string(), "Failed to open direct regular skill file");
    let source = err
        .root_cause()
        .downcast_ref::<std::io::Error>()
        .expect("permission failure must retain its typed I/O source");
    assert!(
        expected_kinds.contains(&source.kind()),
        "unexpected typed I/O error kind: {:?}",
        source.kind()
    );
}

#[test]
fn skill_host_file_errors_keep_fixed_public_category() {
    let dir = tempdir().expect("tempdir");
    let skills = dir.path().join("skills");
    std::fs::create_dir_all(skills.join("demo.md")).expect("non-regular skill path");

    let list_error = list_all_skills_in(std::slice::from_ref(&skills))
        .expect_err("non-regular listed skill must fail closed");
    assert_skill_file_error(
        list_error,
        &[
            std::io::ErrorKind::InvalidData,
            std::io::ErrorKind::PermissionDenied,
        ],
    );

    let get_error = load_skill_by_name_in("demo", std::slice::from_ref(&skills))
        .expect_err("non-regular direct skill must fail closed");
    assert_skill_file_error(
        get_error,
        &[
            std::io::ErrorKind::InvalidData,
            std::io::ErrorKind::PermissionDenied,
        ],
    );
}

#[test]
fn missing_skill_dirs_still_return_empty_results() {
    let dir = tempdir().expect("tempdir");
    let missing = dir.path().join("missing");
    let dirs = vec![missing];

    let listed = list_all_skills_in(&dirs).expect("missing dir is empty");
    let loaded = load_skill_by_name_in("demo", &dirs).expect("missing dir is empty");

    assert!(listed.is_empty());
    assert!(loaded.is_none());
}

#[cfg(unix)]
#[test]
fn list_all_skills_fails_closed_on_unreadable_skill_file() {
    let dir = tempdir().expect("tempdir");
    let skills = dir.path().join("skills");
    std::fs::create_dir_all(&skills).expect("mkdir");
    let skill = skills.join("demo.md");
    std::fs::write(&skill, "---\ndescription: demo\n---\ncontent").expect("write");

    let original = std::fs::metadata(&skill).expect("metadata").permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&skill, blocked).expect("chmod");

    let err = list_all_skills_in(std::slice::from_ref(&skills))
        .expect_err("unreadable skill file must fail closed");

    std::fs::set_permissions(&skill, original).expect("restore perms");
    assert_skill_file_error(err, &[std::io::ErrorKind::PermissionDenied]);
}

#[cfg(unix)]
#[test]
fn get_skill_fails_closed_on_unreadable_skill_file() {
    let dir = tempdir().expect("tempdir");
    let skills = dir.path().join("skills");
    std::fs::create_dir_all(&skills).expect("mkdir");
    let skill = skills.join("demo.md");
    std::fs::write(&skill, "---\ndescription: demo\n---\ncontent").expect("write");

    let original = std::fs::metadata(&skill).expect("metadata").permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&skill, blocked).expect("chmod");

    let err = load_skill_by_name_in("demo", std::slice::from_ref(&skills))
        .expect_err("unreadable skill file must fail closed");

    std::fs::set_permissions(&skill, original).expect("restore perms");
    assert_skill_file_error(err, &[std::io::ErrorKind::PermissionDenied]);
}
