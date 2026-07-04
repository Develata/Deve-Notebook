//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! Bottom bar connection, write-gate, and storage status copy.

use super::super::Locale;

pub fn ready(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ready",
        Locale::Zh => "就绪",
    }
}

pub fn reconnecting(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Reconnecting",
        Locale::Zh => "重连中",
    }
}

pub fn offline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Offline",
        Locale::Zh => "离线",
    }
}

pub fn handshaking_repo(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Handshaking repo...",
        Locale::Zh => "仓库握手中...",
    }
}

pub fn peer_not_registered(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Logged in / Peer not registered",
        Locale::Zh => "已登录 / Peer 未注册",
    }
}

pub fn retry_peer_registration(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Retry peer",
        Locale::Zh => "重试 Peer",
    }
}

pub fn read_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Read-only",
        Locale::Zh => "只读",
    }
}

pub fn snapshot_loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading snapshot...",
        Locale::Zh => "加载快照中...",
    }
}

pub fn pending_ack(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Pending Ack ({count})"),
        Locale::Zh => format!("等待确认 ({count})"),
    }
}

pub fn storage_limited_read_only(locale: Locale, reason: &str) -> String {
    match locale {
        Locale::En => format!("Storage limited ({reason}); read-only mode is active"),
        Locale::Zh => format!("存储受限（{reason}），当前处于只读模式"),
    }
}

pub fn unauthorized(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Session Expired",
        Locale::Zh => "会话已过期",
    }
}

pub fn native_bootstrap_invalid(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native Bootstrap Invalid",
        Locale::Zh => "原生启动参数无效",
    }
}

pub fn native_session_pending(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native Session Pending",
        Locale::Zh => "等待原生会话",
    }
}

pub fn native_service_offline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native Service Offline",
        Locale::Zh => "原生服务离线",
    }
}

pub fn native_reprobe_required(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native Reprobe Required",
        Locale::Zh => "需要重新探测原生会话",
    }
}
