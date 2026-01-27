use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryTree {
    pub root: String,
    pub structure: String, // String representation of the tree
}

impl DirectoryTree {
    /// Generate a compact directory tree for LLM context
    ///
    /// - Respects .gitignore
    /// - Limits depth and file count to save tokens
    pub fn generate(root_path: &Path) -> Self {
        let mut builder = WalkBuilder::new(root_path);
        builder
            .hidden(true) // Skip hidden files (.git) by default usually, but we want to be specific
            .git_ignore(true)
            .git_global(false) // Don't look at user's global gitignore
            .max_depth(Some(5)); // Limit depth

        let walker = builder.build();

        let mut output = String::new();
        output.push_str(&format!("Project Root: {}\n", root_path.display()));

        // Simple tree rendering
        // For a huge repo, we might want a JSON structure, but LLMs often prefer visual trees or list of paths.
        // Let's use a list of relative paths for density.

        let mut count = 0;
        const MAX_FILES: usize = 500;

        for result in walker {
            if count >= MAX_FILES {
                output.push_str("... (truncated)\n");
                break;
            }

            match result {
                Ok(entry) => {
                    if entry.depth() == 0 {
                        continue;
                    }

                    let path = entry.path();
                    // Get relative path
                    if let Ok(_rel) = path.strip_prefix(root_path) {
                        // Fix depth calculation: depth() includes root (depth 0).
                        // If root is ".", then "src" is depth 1.
                        // We want indentation based on depth.
                        // depth 1 ("src") -> indent 0
                        // depth 2 ("src/lib.rs") -> indent 1

                        let prefix = "  ".repeat(entry.depth().saturating_sub(1));
                        let name = entry.file_name().to_string_lossy();
                        let indicator = if path.is_dir() { "/" } else { "" };

                        output.push_str(&format!("{}{}{}\n", prefix, name, indicator));
                        count += 1;
                    }
                }
                Err(_) => continue,
            }
        }

        Self {
            root: root_path.to_string_lossy().to_string(),
            structure: output,
        }
    }
}
