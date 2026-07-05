//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn webdav_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "webdav:push",
        Locale::Zh => "webdav:push",
    }
}

pub fn webdav_pull(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "webdav:pull",
        Locale::Zh => "webdav:pull",
    }
}

pub fn s3_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "s3:push",
        Locale::Zh => "s3:push",
    }
}

pub fn s3_pull(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "s3:pull",
        Locale::Zh => "s3:pull",
    }
}

pub fn remote_projection_backend_intent(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Sends a backend/core remote projection intent; the backend uses the current repo URL as locator, and pull must enter External Changes."
        }
        Locale::Zh => {
            "发送 backend/core remote projection intent；后端使用当前 repo URL 作为 locator，pull 必须进入 External Changes。"
        }
    }
}
