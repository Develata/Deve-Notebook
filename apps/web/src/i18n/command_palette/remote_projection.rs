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

pub fn remote_projection_cli_only_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "CLI/backend-only: pull writes projection files, then External Changes must be confirmed"
        }
        Locale::Zh => {
            "仅 CLI/backend：pull 覆盖 projection 文件，之后必须通过 External Changes 确认"
        }
    }
}
