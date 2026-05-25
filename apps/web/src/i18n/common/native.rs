//! Native shell recovery copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference

use super::Locale;

pub fn native_bootstrap_invalid_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native bootstrap invalid",
        Locale::Zh => "原生启动参数无效",
    }
}

pub fn native_bootstrap_invalid_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The native shell provided an invalid local endpoint. Restart the app or service."
        }
        Locale::Zh => "原生外壳提供了无效的本地端点。请重启应用或本地服务。",
    }
}

pub fn native_session_pending_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native session pending",
        Locale::Zh => "等待原生会话",
    }
}

pub fn native_session_pending_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Waiting for the native shell to bind the local session before loading the workspace."
        }
        Locale::Zh => "正在等待原生外壳绑定本地会话，然后再加载工作区。",
    }
}

pub fn native_service_offline_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native service offline",
        Locale::Zh => "原生服务离线",
    }
}

pub fn native_service_offline_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The embedded local service is unavailable. Restart the native service from the shell."
        }
        Locale::Zh => "嵌入式本地服务不可用。请从原生外壳重启本地服务。",
    }
}

pub fn native_reprobe_required_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native reprobe required",
        Locale::Zh => "需要重新探测原生会话",
    }
}

pub fn native_reprobe_required_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The app returned from background and must revalidate session and workspace state before writing."
        }
        Locale::Zh => "应用从后台恢复后，需要重新验证会话与工作区状态才能写入。",
    }
}
