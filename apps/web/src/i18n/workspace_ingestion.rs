//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! Workspace-ingestion capability and aggregate-health copy.

use super::Locale;

pub fn unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Workspace changes are temporarily unavailable",
        Locale::Zh => "工作区变更暂时不可用",
    }
}

pub fn restart_service(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Restart the service to restore workspace change detection",
        Locale::Zh => "重启服务以恢复工作区变更检测",
    }
}

pub fn blocker(locale: Locale) -> String {
    match locale {
        Locale::En => format!("{}. {}.", unavailable(locale), restart_service(locale)),
        Locale::Zh => format!("{}。{}。", unavailable(locale), restart_service(locale)),
    }
}

pub fn health_healthy(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Workspace ingestion healthy",
        Locale::Zh => "工作区摄取正常",
    }
}

pub fn health_transitioning(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Workspace ingestion transitioning",
        Locale::Zh => "工作区摄取正在切换",
    }
}

pub fn health_degraded(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Workspace ingestion degraded",
        Locale::Zh => "工作区摄取已降级",
    }
}

pub fn health_unknown(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Workspace ingestion status unknown",
        Locale::Zh => "工作区摄取状态未知",
    }
}

pub fn health_counts(locale: Locale, running: u64, expected: u64, unavailable: u64) -> String {
    let running = localized_number(locale, running);
    let expected = localized_number(locale, expected);
    let unavailable = localized_number(locale, unavailable);
    match locale {
        Locale::En => format!("{running}/{expected} running · {unavailable} unavailable"),
        Locale::Zh => format!("运行 {running}/{expected} · 不可用 {unavailable}"),
    }
}

#[cfg(target_arch = "wasm32")]
fn localized_number(locale: Locale, value: u64) -> String {
    use wasm_bindgen::JsValue;

    let locales = js_sys::Array::of1(&JsValue::from_str(locale.as_bcp47()));
    let formatter = js_sys::Intl::NumberFormat::new(&locales, &js_sys::Object::new());
    formatter
        .format()
        .call1(formatter.as_ref(), &JsValue::from_f64(value as f64))
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_else(|| value.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn localized_number(_locale: Locale, value: u64) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ingestion_copy_is_complete_in_both_locales() {
        for locale in [Locale::En, Locale::Zh] {
            for value in [
                unavailable(locale),
                restart_service(locale),
                health_healthy(locale),
                health_transitioning(locale),
                health_degraded(locale),
                health_unknown(locale),
            ] {
                assert!(!value.trim().is_empty());
            }
            let blocker = blocker(locale);
            assert!(blocker.contains(unavailable(locale)));
            assert!(blocker.contains(restart_service(locale)));
            let counts = health_counts(locale, 1, 2, 1);
            assert!(counts.contains("1"));
            assert!(counts.contains("2"));
        }
    }
}
