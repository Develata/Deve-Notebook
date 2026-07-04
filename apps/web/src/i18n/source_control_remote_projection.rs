//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn remote_projection_provider_io_pending_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote projection provider I/O is not wired",
        Locale::Zh => "远端 Projection provider I/O 尚未接线",
    }
}

pub fn remote_projection_provider_io_pending_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "provider_io_ready=false. The web command is admitted only as a backend/core intent; no WebDAV/S3 files were pushed or pulled, and pull still requires External Changes confirmation."
        }
        Locale::Zh => {
            "provider_io_ready=false。Web 命令只作为 backend/core intent 接入；未执行 WebDAV/S3 文件 push/pull，pull 仍必须经 External Changes 确认。"
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
