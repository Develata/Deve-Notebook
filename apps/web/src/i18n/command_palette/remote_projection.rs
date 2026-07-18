//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn webdav_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Projection: WebDAV Push",
        Locale::Zh => "远程投影：WebDAV 推送",
    }
}

pub fn s3_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Projection: S3 Push",
        Locale::Zh => "远程投影：S3 推送",
    }
}

pub fn remote_projection_backend_intent(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Sends a typed backend Remote Projection push intent for the exact current repository scope."
        }
        Locale::Zh => "为当前精确仓库作用域发送强类型后端 Remote Projection 推送意图。",
    }
}

pub fn remote_projection_scope_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: current repository scope is not ready",
        Locale::Zh => "不可用：当前仓库作用域尚未就绪",
    }
}

pub fn remote_projection_push_succeeded(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Projection push completed",
        Locale::Zh => "远程投影推送已完成",
    }
}
