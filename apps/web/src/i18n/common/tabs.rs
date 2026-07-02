//! Common tab and surface copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference

use super::Locale;

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
        assert_eq!(document_tab(Locale::Zh), "文档标签页");
        assert_eq!(diff_tab(Locale::En), "Diff tab");
        assert_eq!(diff_tab(Locale::Zh), "差异标签页");
        assert_eq!(close_tab(Locale::Zh), "关闭标签页");
        assert_eq!(documents(Locale::Zh), "文档");
        assert_eq!(diffs(Locale::En), "Diffs");
    }

    #[test]
    fn open_tab_labels_are_localized() {
        assert_eq!(open_tabs(Locale::En), "Open tabs");
        assert_eq!(open_tabs(Locale::Zh), "已打开标签页");
        assert_eq!(switch_open_tabs(Locale::Zh), "切换已打开标签页");
    }
}
