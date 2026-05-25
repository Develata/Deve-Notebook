// apps\web\src\i18n
//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Source Control Native Recovery

use super::Locale;

pub fn native_bootstrap_invalid_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Restart the native shell or local service before using Source Control.",
        Locale::Zh => "请先重启原生外壳或本地服务，再使用源代码管理。",
    }
}

pub fn native_session_pending_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Wait for the native shell to bind the local session before changing Source Control state."
        }
        Locale::Zh => "请等待原生外壳绑定本地会话后再修改源代码管理状态。",
    }
}

pub fn native_service_offline_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Start the embedded native service before staging, discarding, or committing changes."
        }
        Locale::Zh => "请先启动嵌入式原生服务，再暂存、放弃或提交更改。",
    }
}

pub fn native_reprobe_required_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Wait for foreground session reprobe to finish before changing Source Control state."
        }
        Locale::Zh => "请等待前台会话重新探测完成后再修改源代码管理状态。",
    }
}
