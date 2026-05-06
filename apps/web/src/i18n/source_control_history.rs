// apps\web\src\i18n
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!
//! Source Control history panel strings.

use super::Locale;

pub fn history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "History",
        Locale::Zh => "历史记录",
    }
}

pub fn history_compare_message(locale: Locale, base_label: &str, target_label: &str) -> String {
    match locale {
        Locale::En => format!("Comparing {base_label} -> {target_label}."),
        Locale::Zh => format!("正在比较 {base_label} -> {target_label}。"),
    }
}

pub fn history_base_selected_message(locale: Locale, base_label: &str) -> String {
    match locale {
        Locale::En => format!("Base {base_label} selected. Click another commit to compare."),
        Locale::Zh => format!("已选择基准提交 {base_label}。点击另一条提交即可比较。"),
    }
}

pub fn history_selected_target_message(locale: Locale, target_label: &str) -> String {
    match locale {
        Locale::En => format!("Selected {target_label}. Use it as the comparison base?"),
        Locale::Zh => format!("已选择提交 {target_label}。要把它设为比较基准吗？"),
    }
}

pub fn clear_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Clear",
        Locale::Zh => "清除",
    }
}

pub fn use_as_base_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Use as Base",
        Locale::Zh => "设为基准",
    }
}

pub fn loading_history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading history...",
        Locale::Zh => "正在加载历史记录...",
    }
}

pub fn no_commit_history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No commit history yet on this branch.",
        Locale::Zh => "这个分支上还没有提交历史。",
    }
}

pub fn no_diff_between_commits(locale: Locale, base: &str, target: &str) -> String {
    match locale {
        Locale::En => format!("No file-level diff available between {base} and {target}."),
        Locale::Zh => format!("提交 {base} 与 {target} 之间没有可展示的文件级差异。"),
    }
}

pub fn no_diff_for_commit(locale: Locale) -> String {
    match locale {
        Locale::En => "No file-level diff available for this commit.".to_string(),
        Locale::Zh => "这个提交没有可展示的文件级差异。".to_string(),
    }
}

pub fn history_base_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Base",
        Locale::Zh => "基准",
    }
}

pub fn history_target_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Target",
        Locale::Zh => "目标",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_control_history_copy_is_localized() {
        assert_eq!(loading_history(Locale::Zh), "正在加载历史记录...");
        assert_eq!(
            history_compare_message(Locale::En, "abc1234", "def5678"),
            "Comparing abc1234 -> def5678."
        );
        assert_eq!(
            no_diff_between_commits(Locale::En, "abc1234", "def5678"),
            "No file-level diff available between abc1234 and def5678."
        );
    }
}
