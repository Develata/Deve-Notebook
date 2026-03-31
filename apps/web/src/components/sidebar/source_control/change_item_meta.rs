use deve_core::source_control::{ChangeEntry, ChangeStatus};

pub struct ChangeItemMeta {
    pub display_name: String,
    pub directory: String,
    pub file_icon_class: &'static str,
    pub icon_char: &'static str,
    pub color_class: &'static str,
}

pub fn build_change_item_meta(entry: &ChangeEntry) -> ChangeItemMeta {
    let path_parts: Vec<&str> = entry.path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();
    let display_name = entry
        .renamed_from
        .as_ref()
        .and_then(|old_path| old_path.rsplit('/').next())
        .map(|old_name| format!("{} -> {}", old_name, filename))
        .unwrap_or_else(|| filename.clone());
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len() - 1].join("/")
    } else {
        String::new()
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
