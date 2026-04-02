use crate::i18n::Locale;

pub fn compare_message(locale: Locale, base_label: &str, target_label: &str) -> String {
    match locale {
        Locale::En => format!("Comparing {base_label} -> {target_label}."),
        Locale::Zh => format!("正在比较 {base_label} -> {target_label}。"),
    }
}

pub fn base_selected_message(locale: Locale, base_label: &str) -> String {
    match locale {
        Locale::En => format!("Base {base_label} selected. Click another commit to compare."),
        Locale::Zh => format!("已选择基准提交 {base_label}。点击另一条提交即可比较。"),
    }
}

pub fn selected_target_message(locale: Locale, target_label: &str) -> String {
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
