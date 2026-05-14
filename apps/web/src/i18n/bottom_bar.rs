// apps\web\src\i18n
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!
//! # I18n Bottom Bar Module (底部栏翻译)

use super::Locale;

pub fn branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Branch",
        Locale::Zh => "分支",
    }
}

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

pub fn chars(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Chars",
        Locale::Zh => "字符",
    }
}

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

pub fn loading_progress(locale: Locale, done: usize, total: usize, eta_ms: u64) -> String {
    match locale {
        Locale::En => {
            if eta_ms > 0 {
                format!("Loading {}/{} (~{}ms)", done, total, eta_ms)
            } else {
                format!("Loading {}/{}", done, total)
            }
        }
        Locale::Zh => {
            if eta_ms > 0 {
                format!("加载中 {}/{} (~{}ms)", done, total, eta_ms)
            } else {
                format!("加载中 {}/{}", done, total)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::toggle_status_details;
    use crate::i18n::Locale;

    #[test]
    fn mobile_i18n_bottom_bar_toggle_copy_has_facade_key() {
        assert_eq!(toggle_status_details(Locale::En), "Toggle status details");
    }
}
