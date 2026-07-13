//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! Bottom bar connection, write-gate, and storage status copy.

use super::super::Locale;
use crate::storage::BrowserIdentityBlocker;

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

pub fn editor_sync_error(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Editor sync error",
        Locale::Zh => "编辑器同步错误",
    }
}

pub fn pending_ack(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Pending Ack ({count})"),
        Locale::Zh => format!("等待确认 ({count})"),
    }
}

pub fn storage_limited_read_only(locale: Locale, blocker: BrowserIdentityBlocker) -> &'static str {
    match (locale, blocker) {
        (Locale::En, BrowserIdentityBlocker::WebCryptoUnavailable) => {
            "Browser cryptography is unavailable; read-only mode is active. Update this browser or Android System WebView."
        }
        (Locale::Zh, BrowserIdentityBlocker::WebCryptoUnavailable) => {
            "浏览器加密能力不可用，当前保持只读。请更新浏览器或 Android System WebView。"
        }
        (Locale::En, BrowserIdentityBlocker::IndexedDbUnavailable) => {
            "Persistent browser storage is unavailable; read-only mode is active. Allow site storage or use a supported browser."
        }
        (Locale::Zh, BrowserIdentityBlocker::IndexedDbUnavailable) => {
            "浏览器持久存储不可用，当前保持只读。请允许站点存储或改用受支持的浏览器。"
        }
        (Locale::En, BrowserIdentityBlocker::Ed25519Unavailable) => {
            "WebCrypto Ed25519 is unavailable; read-only mode is active. Update this browser or Android System WebView."
        }
        (Locale::Zh, BrowserIdentityBlocker::Ed25519Unavailable) => {
            "WebCrypto Ed25519 不可用，当前保持只读。请更新浏览器或 Android System WebView。"
        }
        (Locale::En, BrowserIdentityBlocker::CapabilityProbeFailed) => {
            "Browser identity capability check failed; read-only mode is active. Retry or update this browser."
        }
        (Locale::Zh, BrowserIdentityBlocker::CapabilityProbeFailed) => {
            "浏览器身份能力探测失败，当前保持只读。请重试或更新浏览器。"
        }
        (Locale::En, BrowserIdentityBlocker::IdentityRecoveryFailed) => {
            "Browser identity could not be restored; read-only mode is active. Retry peer registration."
        }
        (Locale::Zh, BrowserIdentityBlocker::IdentityRecoveryFailed) => {
            "浏览器身份无法恢复，当前保持只读。请重试 Peer 注册。"
        }
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
