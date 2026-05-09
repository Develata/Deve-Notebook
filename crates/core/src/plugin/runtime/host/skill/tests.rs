use super::{list_all_skills_in, load_skill_by_name_in};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

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
    assert!(err.to_string().contains("Failed to load skill from"));
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
    assert!(err.to_string().contains("Permission denied"));
}
