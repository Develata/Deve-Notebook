//! plan_ref:
//!   - 19_plugins#skills-cli-extension-boundary

use crate::plugin::resource_budget::{
    MAX_SKILL_COUNT, MAX_SKILL_FILE_BYTES, MAX_SKILL_TOTAL_BYTES,
    read_utf8_handle_bounded_and_verify,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

const MAX_SKILL_NAME_BYTES: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String, // The actual prompt/instructions
    pub path: PathBuf,   // File location
}

pub struct SkillManager {
    skills_dir: PathBuf,
}

impl SkillManager {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }

    /// Load a specific skill by name
    pub fn get(&self, name: &str) -> Result<Option<Skill>> {
        validate_skill_name(name)?;
        let path = self.skills_dir.join(format!("{}.md", name));
        let Some(file) = self.open_skill_file(&path)? else {
            return Ok(None);
        };
        self.load_skill_from_file(&path, file).map(Some)
    }

    /// List all available skills
    pub fn list(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();
        if !self
            .skills_dir
            .try_exists()
            .with_context(|| format!("Failed to stat skills directory: {:?}", self.skills_dir))?
        {
            return Ok(skills);
        }

        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                paths.push(path);
                if paths.len() > MAX_SKILL_COUNT {
                    anyhow::bail!("Skill directory exceeds the configured skill-count budget");
                }
            }
        }
        paths.sort();
        let mut total_bytes = 0usize;
        for path in paths {
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Skill filename must be valid UTF-8"))?;
            validate_skill_name(name)?;
            let file = self
                .open_skill_file(&path)?
                .ok_or_else(|| anyhow::anyhow!("Skill file disappeared during bounded lookup"))?;
            let skill = self
                .load_skill_from_file(&path, file)
                .context("Failed to load skill file")?;
            total_bytes = total_bytes
                .checked_add(skill.description.len())
                .and_then(|bytes| bytes.checked_add(skill.content.len()))
                .ok_or_else(|| anyhow::anyhow!("Skill aggregate size overflow"))?;
            if total_bytes > MAX_SKILL_TOTAL_BYTES {
                anyhow::bail!("Skill directory exceeds the aggregate text budget");
            }
            skills.push(skill);
        }
        Ok(skills)
    }

    fn open_skill_file(&self, path: &Path) -> Result<Option<File>> {
        match crate::utils::fs::open_regular_file_read(path, "skill file") {
            Ok(file) => Ok(Some(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).with_context(|| "Failed to open direct regular skill file"),
        }
    }

    fn load_skill_from_file(&self, path: &Path, file: File) -> Result<Skill> {
        let raw = read_utf8_handle_bounded_and_verify(
            file,
            path,
            MAX_SKILL_FILE_BYTES,
            "skill Markdown",
        )?;

        // Simple frontmatter parsing
        // Look for --- ... --- block
        let mut lines = raw.lines();
        let mut description = String::new();
        let mut content = String::new();
        let mut in_frontmatter = false;

        if let Some(first) = lines.next() {
            if first.trim() == "---" {
                in_frontmatter = true;
            } else {
                content.push_str(first);
                content.push('\n');
            }
        }

        for line in lines {
            if in_frontmatter {
                if line.trim() == "---" {
                    in_frontmatter = false;
                    continue;
                }
                // Parse key: value
                if let Some((key, val)) = line.split_once(':')
                    && key.trim() == "description"
                {
                    description = val.trim().to_string();
                }
            } else {
                content.push_str(line);
                content.push('\n');
            }
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(Skill {
            name,
            description,
            content: content.trim().to_string(),
            path: path.to_path_buf(),
        })
    }
}

fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > MAX_SKILL_NAME_BYTES
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        anyhow::bail!("Skill name must match [A-Za-z0-9_-] and stay within the length budget");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SkillManager;

    #[test]
    fn skill_manager_accepts_direct_ascii_markdown_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("review_notes.md"),
            "---\ndescription: Review\n---\nSafe content",
        )
        .expect("skill");
        let skill = SkillManager::new(dir.path().to_path_buf())
            .get("review_notes")
            .expect("lookup")
            .expect("skill");
        assert_eq!(skill.name, "review_notes");
        assert_eq!(skill.description, "Review");
        assert_eq!(skill.content, "Safe content");
    }

    #[test]
    fn skill_manager_rejects_path_traversal_and_symlink_escape() {
        let root = tempfile::tempdir().expect("root");
        let skills = root.path().join("skills");
        std::fs::create_dir(&skills).expect("skills");
        std::fs::write(root.path().join("secret.md"), "outside").expect("outside");
        let manager = SkillManager::new(skills.clone());
        for invalid in [
            "../secret",
            "..",
            ".",
            "nested/name",
            "nested\\name",
            "技能",
        ] {
            assert!(manager.get(invalid).is_err(), "{invalid:?}");
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(root.path().join("secret.md"), skills.join("linked.md"))
            .expect("symlink");
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(
            root.path().join("secret.md"),
            skills.join("linked.md"),
        )
        .is_err()
        {
            return;
        }

        assert!(manager.get("linked").is_err());
        assert!(manager.list().is_err());
    }

    #[test]
    fn skill_manager_rejects_oversized_markdown() {
        let dir = tempfile::tempdir().expect("root");
        std::fs::write(
            dir.path().join("large.md"),
            vec![b'x'; (super::MAX_SKILL_FILE_BYTES + 1) as usize],
        )
        .expect("large skill");

        let error = SkillManager::new(dir.path().to_path_buf())
            .get("large")
            .expect_err("oversized skill must fail closed");

        assert!(error.to_string().contains("resource budget"));
    }

    #[test]
    fn skill_manager_rejects_count_over_aggregate_budget() {
        let dir = tempfile::tempdir().expect("root");
        for index in 0..=super::MAX_SKILL_COUNT {
            std::fs::write(dir.path().join(format!("skill_{index:03}.md")), "x").expect("skill");
        }

        let error = SkillManager::new(dir.path().to_path_buf())
            .list()
            .expect_err("skill count over budget must fail closed");

        assert!(error.to_string().contains("skill-count budget"));
    }

    #[test]
    fn skill_manager_rejects_total_text_over_aggregate_budget() {
        let dir = tempfile::tempdir().expect("root");
        let per_skill = super::MAX_SKILL_FILE_BYTES as usize;
        let skill_count = super::MAX_SKILL_TOTAL_BYTES / per_skill + 1;
        for index in 0..skill_count {
            std::fs::write(
                dir.path().join(format!("skill_{index:03}.md")),
                vec![b'x'; per_skill],
            )
            .expect("skill");
        }

        let error = SkillManager::new(dir.path().to_path_buf())
            .list()
            .expect_err("skill aggregate text over budget must fail closed");

        assert!(error.to_string().contains("aggregate text budget"));
    }
}
