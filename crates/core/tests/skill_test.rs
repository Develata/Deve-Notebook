#[cfg(test)]
mod tests {
    use deve_core::skill::SkillManager;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    #[cfg(unix)]
    use tempfile::tempdir;

    #[test]
    fn test_skill_loading() {
        let skills_dir = PathBuf::from("tests/skills");
        if !skills_dir.exists() {
            // Setup for CI/clean env if needed
            return;
        }

        let manager = SkillManager::new(skills_dir);
        let skills = manager.list().expect("Failed to list skills");

        assert!(!skills.is_empty());

        let target = skills.iter().find(|s| s.name == "test-skill");
        assert!(target.is_some());

        let skill = target.unwrap();
        assert_eq!(skill.description, "A test skill for verifying the loader");
        assert!(skill.content.contains("# Test Skill"));
    }

    #[cfg(unix)]
    #[test]
    fn skill_listing_fails_closed_on_unreadable_skill_file() {
        let dir = tempdir().expect("tempdir");
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).expect("mkdir");
        std::fs::write(
            skills_dir.join("good.md"),
            "---\ndescription: good\n---\n# Good Skill\n",
        )
        .expect("write good");
        let bad = skills_dir.join("bad.md");
        std::fs::write(&bad, "---\ndescription: bad\n---\n# Bad Skill\n").expect("write bad");
        let original = std::fs::metadata(&bad).expect("metadata").permissions();
        let mut perms = original.clone();
        perms.set_mode(0o000);
        std::fs::set_permissions(&bad, perms).expect("chmod 000");

        let manager = SkillManager::new(skills_dir);
        let err = manager
            .list()
            .expect_err("unreadable skill file must fail closed");

        std::fs::set_permissions(&bad, original).expect("restore perms");
        assert_eq!(err.to_string(), "Failed to open direct regular skill file");
        let io_error = err
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .expect("skill failure keeps typed I/O source");
        assert_eq!(io_error.kind(), std::io::ErrorKind::PermissionDenied);
    }
}
