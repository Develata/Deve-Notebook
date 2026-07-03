//! External Changes UI copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 12_source_control_ui#external-changes-sibling-view

use super::Locale;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "External Changes",
        Locale::Zh => "外部修改",
    }
}

pub fn pending(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "External Changes",
        Locale::Zh => "外部修改",
    }
}

pub fn staged(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Staged External Changes",
        Locale::Zh => "已暂存外部修改",
    }
}

pub fn no_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No external changes",
        Locale::Zh => "没有外部修改",
    }
}

pub fn apply_to_ledger(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Apply to Ledger",
        Locale::Zh => "确认外部修改",
    }
}

pub fn apply_to_ledger_disabled(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage external changes before applying to ledger",
        Locale::Zh => "请先暂存外部修改再确认写入账本",
    }
}

pub fn overlap_blocked(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Overlaps confirmed ledger changes",
        Locale::Zh => "与已确认账本更改重叠",
    }
}

pub fn open_diff(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Diff",
        Locale::Zh => "打开差异",
    }
}

pub fn stage(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage",
        Locale::Zh => "暂存",
    }
}

pub fn unstage(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unstage",
        Locale::Zh => "取消暂存",
    }
}

pub fn discard(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard External Change",
        Locale::Zh => "放弃外部修改",
    }
}
