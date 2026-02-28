// apps\web\src\i18n
//! # I18n Merge Module (合并翻译)
//!
//! 手动合并模式、待处理操作面板相关翻译。

use super::Locale;

pub fn pending_merges(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Pending Merges",
        Locale::Zh => "待合并",
    }
}

pub fn manual_mode_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Manual Mode is active. These changes from peers are waiting for your approval.",
        Locale::Zh => "手动模式已激活。来自对等方的更改等待您的审批。",
    }
}

pub fn current(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Current",
        Locale::Zh => "当前",
    }
}

pub fn incoming(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Incoming",
        Locale::Zh => "传入",
    }
}

pub fn no_pending(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No pending operations.",
        Locale::Zh => "暂无待处理操作。",
    }
}

pub fn discard_all(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard All",
        Locale::Zh => "全部丢弃",
    }
}

pub fn merge_n_ops(locale: Locale, n: u32) -> String {
    match locale {
        Locale::En => format!("Merge {} Operations", n),
        Locale::Zh => format!("合并 {} 个操作", n),
    }
}

pub fn sync_mode_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sync Mode:",
        Locale::Zh => "同步模式:",
    }
}

pub fn manual(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Manual",
        Locale::Zh => "手动",
    }
}

pub fn auto(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Auto",
        Locale::Zh => "自动",
    }
}

pub fn n_pending(locale: Locale, n: u32) -> String {
    match locale {
        Locale::En => format!("{} pending", n),
        Locale::Zh => format!("{} 待处理", n),
    }
}

pub fn pending_operations(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Pending Operations",
        Locale::Zh => "待处理操作",
    }
}

pub fn confirm_merge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirm Merge",
        Locale::Zh => "确认合并",
    }
}

pub fn discard(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard",
        Locale::Zh => "丢弃",
    }
}
