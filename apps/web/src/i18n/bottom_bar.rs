// apps\web\src\i18n
//! # I18n Bottom Bar Module (底部栏翻译)

use super::Locale;

pub fn words(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Words",
        Locale::Zh => "字数",
    }
}

pub fn lines(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Lines",
        Locale::Zh => "行数",
    }
}

pub fn col(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Col",
        Locale::Zh => "列",
    }
}

pub fn ready(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ready",
        Locale::Zh => "就绪",
    }
}

pub fn syncing(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Syncing...",
        Locale::Zh => "同步中...",
    }
}

pub fn offline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Offline",
        Locale::Zh => "离线",
    }
}

pub fn toggle_status_details(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle status details",
        Locale::Zh => "切换状态详情",
    }
}

pub fn first(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "First",
        Locale::Zh => "最前",
    }
}

pub fn prev(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Prev",
        Locale::Zh => "上一步",
    }
}

pub fn next(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Next",
        Locale::Zh => "下一步",
    }
}

pub fn latest(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Latest",
        Locale::Zh => "最新",
    }
}

pub fn time_travel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Time Travel",
        Locale::Zh => "时间回放",
    }
}

pub fn loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading...",
        Locale::Zh => "加载中...",
    }
}
