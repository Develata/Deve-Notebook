//! Common tab and surface copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference

use super::Locale;

pub fn tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Tab",
        Locale::Zh => "制表",
    }
}

pub fn document_tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Document tab",
        Locale::Zh => "文档标签页",
    }
}

pub fn diff_tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff tab",
        Locale::Zh => "差异标签页",
    }
}

pub fn document_surface(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Document",
        Locale::Zh => "文档",
    }
}

pub fn diff_surface(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff",
        Locale::Zh => "差异",
    }
}

pub fn close_tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close tab",
        Locale::Zh => "关闭标签页",
    }
}

pub fn open_tabs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open tabs",
        Locale::Zh => "已打开标签页",
    }
}

pub fn open_tabs_count(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En if count == 1 => format!("{count} open tab"),
        Locale::En => format!("{count} open tabs"),
        Locale::Zh => format!("已打开 {count} 个标签页"),
    }
}

pub fn switch_open_tabs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switch open tabs",
        Locale::Zh => "切换已打开标签页",
    }
}

pub fn documents(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Documents",
        Locale::Zh => "文档",
    }
}

pub fn diffs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diffs",
        Locale::Zh => "差异",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_surface_labels_are_localized() {
        assert_eq!(tab(Locale::Zh), "制表");
        assert_eq!(document_tab(Locale::Zh), "文档标签页");
        assert_eq!(diff_tab(Locale::En), "Diff tab");
        assert_eq!(diff_tab(Locale::Zh), "差异标签页");
        assert_eq!(document_surface(Locale::En), "Document");
        assert_eq!(document_surface(Locale::Zh), "文档");
        assert_eq!(diff_surface(Locale::En), "Diff");
        assert_eq!(diff_surface(Locale::Zh), "差异");
        assert_eq!(close_tab(Locale::Zh), "关闭标签页");
        assert_eq!(documents(Locale::Zh), "文档");
        assert_eq!(diffs(Locale::En), "Diffs");
    }

    #[test]
    fn open_tab_labels_are_localized_and_counted() {
        assert_eq!(open_tabs(Locale::En), "Open tabs");
        assert_eq!(open_tabs(Locale::Zh), "已打开标签页");
        assert_eq!(open_tabs_count(Locale::En, 1), "1 open tab");
        assert_eq!(open_tabs_count(Locale::En, 2), "2 open tabs");
        assert_eq!(open_tabs_count(Locale::Zh, 2), "已打开 2 个标签页");
        assert_eq!(switch_open_tabs(Locale::Zh), "切换已打开标签页");
    }
}
