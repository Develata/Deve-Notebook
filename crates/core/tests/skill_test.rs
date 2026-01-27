#[cfg(test)]
mod tests {
    use deve_core::skill::SkillManager;
    use std::path::PathBuf;

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
}
