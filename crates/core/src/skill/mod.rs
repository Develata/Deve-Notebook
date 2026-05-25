//! plan_ref:
//!   - 19_plugins#skills-cli-extension-boundary

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        // Assume skill file is name.md
        let path = self.skills_dir.join(format!("{}.md", name));
        if !path
            .try_exists()
            .with_context(|| format!("Failed to stat skill path: {:?}", path))?
        {
            return Ok(None);
        }

        self.load_skill_from_path(&path).map(Some)
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

        for entry in std::fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                skills.push(
                    self.load_skill_from_path(&path)
                        .with_context(|| format!("Failed to load skill from {:?}", path))?,
                );
            }
        }
        Ok(skills)
    }

    fn load_skill_from_path(&self, path: &PathBuf) -> Result<Skill> {
        let raw = std::fs::read_to_string(path)?;

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
            path: path.clone(),
        })
    }
}
