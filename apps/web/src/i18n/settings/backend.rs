//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 15_settings#native-host-local-backend-preference

use crate::i18n::Locale;

pub fn backend_section(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Backend",
        Locale::Zh => "后端",
    }
}

pub fn backend_section_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native shells can use a local service or a validated remote origin.",
        Locale::Zh => "Native 壳层可以使用本机服务或已校验的远端 origin。",
    }
}

pub fn local_backend_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local Backend",
        Locale::Zh => "本地后端",
    }
}

pub fn remote_backend_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Backend",
        Locale::Zh => "远端后端",
    }
}

pub fn remote_backend_url_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote HTTPS origin",
        Locale::Zh => "远端 HTTPS origin",
    }
}

pub fn validate_and_save_remote(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Validate & Save",
        Locale::Zh => "校验并保存",
    }
}

pub fn validating_remote_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Validating remote backend...",
        Locale::Zh => "正在校验远端后端...",
    }
}

pub fn remote_backend_saved(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote backend saved. The native shell will load that origin.",
        Locale::Zh => "远端后端已保存。Native 壳层将加载该 origin。",
    }
}

pub fn remote_backend_requires_validation(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote URL must pass native validation before it can be saved.",
        Locale::Zh => "远端 URL 必须先通过 native 校验才能保存。",
    }
}

pub fn native_backend_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Native backend controls are unavailable here. In Desktop RemoteBrowser, use the app's native menu or tray to return to LocalBackend."
        }
        Locale::Zh => {
            "此处不提供 native 后端控制。Desktop RemoteBrowser 请使用应用原生菜单或托盘切回 LocalBackend。"
        }
    }
}
