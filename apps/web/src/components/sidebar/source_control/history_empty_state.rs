use crate::components::sidebar::source_control::history_compare_logic::short_commit_id;
use crate::i18n::Locale;

pub fn no_diff_message(
    locale: Locale,
    compare_base_commit_id: Option<&str>,
    target_commit_id: &str,
) -> String {
    if let Some(base_commit_id) = compare_base_commit_id {
        let base = short_commit_id(base_commit_id);
        let target = short_commit_id(target_commit_id);
        return match locale {
            Locale::En => format!("No file-level diff available between {base} and {target}."),
            Locale::Zh => format!("提交 {base} 与 {target} 之间没有可展示的文件级差异。"),
        };
    }

    match locale {
        Locale::En => "No file-level diff available for this commit.".to_string(),
        Locale::Zh => "这个提交没有可展示的文件级差异。".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::no_diff_message;
    use crate::i18n::Locale;

    #[test]
    fn compare_mode_mentions_both_commits() {
        let message = no_diff_message(Locale::En, Some("387cc45abc"), "8175903def");
        assert_eq!(
            message,
            "No file-level diff available between 387cc45 and 8175903."
        );
    }

    #[test]
    fn single_commit_mode_keeps_original_copy() {
        let message = no_diff_message(Locale::En, None, "8175903def");
        assert_eq!(message, "No file-level diff available for this commit.");
    }
}
