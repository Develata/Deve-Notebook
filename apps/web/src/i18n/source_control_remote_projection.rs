//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn remote_projection_provider_io_pending_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote projection provider I/O did not complete",
        Locale::Zh => "远端 Projection provider I/O 未完成",
    }
}

pub fn remote_projection_provider_io_pending_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "provider_io_ready=false. No WebDAV/S3 files were pushed or pulled. Configure a matching repo URL or retry after fixing the provider; pull still requires External Changes confirmation."
        }
        Locale::Zh => {
            "provider_io_ready=false。未执行 WebDAV/S3 文件 push/pull。请配置匹配的 repo URL 或修复 provider 后重试；pull 仍必须经 External Changes 确认。"
        }
    }
}

pub fn remote_projection_session_unavailable_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote projection command was not sent",
        Locale::Zh => "远端 Projection 命令未发送",
    }
}

pub fn remote_projection_session_unavailable_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The browser has no active backend session, so no WebDAV/S3 files were pushed or pulled. Reconnect before running the command; pull still requires External Changes confirmation."
        }
        Locale::Zh => {
            "浏览器当前没有可用 backend session，因此未执行 WebDAV/S3 文件 push/pull。请重新连接后再运行命令；pull 仍必须经 External Changes 确认。"
        }
    }
}
