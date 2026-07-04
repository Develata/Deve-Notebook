//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn source_control_sync(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control: Sync",
        Locale::Zh => "Source Control: 同步",
    }
}

pub fn source_control_commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control: Commit",
        Locale::Zh => "Source Control: 提交",
    }
}

pub fn source_control_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control: Push",
        Locale::Zh => "Source Control: 推送",
    }
}

pub fn source_control_panel_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Unavailable: use the Source Control panel for scoped state and message input"
        }
        Locale::Zh => "不可用：请在源代码管理面板中使用带作用域与提交信息的操作",
    }
}
