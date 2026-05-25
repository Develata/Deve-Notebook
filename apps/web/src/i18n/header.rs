// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Header Module (头部翻译)
//!
//! 包含顶部导航栏相关的翻译字符串。

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

pub fn settings(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Settings",
        Locale::Zh => "设置",
    }
}

pub fn home(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Home",
        Locale::Zh => "首页",
    }
}

pub fn open(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Index",
        Locale::Zh => "打开目录",
    }
}

pub fn command(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Command Palette",
        Locale::Zh => "命令面板",
    }
}

pub fn logout(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign out",
        Locale::Zh => "退出登录",
    }
}

pub fn file_tree(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "File tree",
        Locale::Zh => "文件树",
    }
}

pub fn toggle_outline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle Outline",
        Locale::Zh => "切换大纲",
    }
}

#[cfg(test)]
mod tests {
    use super::{command, file_tree, home, logout, open, toggle_outline};
    use crate::i18n::Locale;

    #[test]
    fn mobile_i18n_header_action_copy_has_facade_keys() {
        assert_eq!(file_tree(Locale::En), "File tree");
        assert_eq!(home(Locale::En), "Home");
        assert_eq!(open(Locale::En), "Open Index");
        assert_eq!(command(Locale::En), "Command Palette");
        assert_eq!(logout(Locale::En), "Sign out");
        assert_eq!(toggle_outline(Locale::En), "Toggle Outline");
    }
}
