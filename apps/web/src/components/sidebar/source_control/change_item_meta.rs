//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
use deve_core::source_control::{ChangeEntry, ChangeStatus};

pub struct ChangeItemMeta {
    pub display_name: String,
    pub directory: String,
    pub file_icon_class: &'static str,
    pub icon_char: &'static str,
    pub color_class: &'static str,
}

fn split_path(path: &str) -> (String, String) {
    let path_parts: Vec<&str> = path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len() - 1].join("/")
    } else {
        String::new()
    };
    (filename, directory)
}

fn root_label(directory: &str) -> &str {
    if directory.is_empty() { "/" } else { directory }
}

pub fn build_change_item_meta(entry: &ChangeEntry) -> ChangeItemMeta {
    let (filename, directory) = split_path(&entry.path);
    let (display_name, directory) = match entry.renamed_from.as_deref() {
        Some(old_path) => {
            let (old_name, old_directory) = split_path(old_path);
            let display_name = if old_name == filename {
                filename.clone()
            } else {
                format!("{old_name} -> {filename}")
            };
            let directory = if old_directory == directory {
                directory.clone()
            } else {
                format!(
                    "{} -> {}",
                    root_label(&old_directory),
                    root_label(&directory)
                )
            };
            (display_name, directory)
        }
        None => (filename.clone(), directory),
    };
    let file_icon_class = if filename.ends_with(".rs") {
        "text-[var(--color-file-rust)]"
    } else {
        "text-muted"
    };
    let (icon_char, color_class) = match entry.status {
        ChangeStatus::Modified => ("M", "text-modified"),
        ChangeStatus::Added if entry.renamed_from.is_some() => ("R", "text-added"),
        ChangeStatus::Added => ("A", "text-added"),
        ChangeStatus::Deleted => ("D", "text-deleted"),
        ChangeStatus::Renamed => ("R", "text-added"),
    };

    ChangeItemMeta {
        display_name,
        directory,
        file_icon_class,
        icon_char,
        color_class,
    }
}

#[cfg(test)]
mod tests {
    use super::build_change_item_meta;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    fn entry(path: &str, renamed_from: Option<&str>) -> ChangeEntry {
        ChangeEntry {
            path: path.into(),
            renamed_from: renamed_from.map(str::to_string),
            doc_id: None,
            status: ChangeStatus::Renamed,
            has_conflict: false,
        }
    }

    #[test]
    fn pure_move_keeps_filename_and_shows_directory_transition() {
        let meta = build_change_item_meta(&entry("archive/note.md", Some("drafts/note.md")));
        assert_eq!(meta.display_name, "note.md");
        assert_eq!(meta.directory, "drafts -> archive");
    }

    #[test]
    fn rename_and_move_show_both_filename_and_directory_changes() {
        let meta = build_change_item_meta(&entry("notes/new.md", Some("drafts/old.md")));
        assert_eq!(meta.display_name, "old.md -> new.md");
        assert_eq!(meta.directory, "drafts -> notes");
    }
}
