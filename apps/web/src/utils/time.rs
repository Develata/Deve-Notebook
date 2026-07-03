//! # 时间格式化工具 (Time Formatting)
//! plan_ref:
//!   - 13_i18n#i18n-facade-contract
//!
//! 在 WASM 环境中使用 `js_sys::Date` 获取当前时间，
//! 将毫秒时间戳转换为相对时间显示。

use crate::i18n::{Locale, t};
use wasm_bindgen::JsValue;

/// 将毫秒时间戳转换为相对时间字符串
///
/// 例: "刚刚", "3 分钟前", "2 小时前", "昨天", "3 天前"
pub fn format_relative(timestamp_ms: i64, locale: Locale) -> String {
    let now_ms = js_sys::Date::now() as i64;
    let diff_secs = (now_ms - timestamp_ms) / 1000;

    if diff_secs < 0 {
        return t::time::just_now(locale).to_string();
    }

    let minutes = diff_secs / 60;
    let hours = diff_secs / 3600;
    let days = diff_secs / 86400;

    match diff_secs {
        0..=59 => t::time::just_now(locale).to_string(),
        60..=3599 => t::time::minutes_ago(locale, minutes),
        3600..=86399 => t::time::hours_ago(locale, hours),
        86400..=172799 => t::time::yesterday(locale).to_string(),
        172800..=604799 => t::time::days_ago(locale, days),
        _ => {
            let date = js_sys::Date::new(&(timestamp_ms as f64).into());
            date.to_locale_date_string(t::time::date_locale(locale), &JsValue::UNDEFINED)
                .as_string()
                .unwrap_or_else(|| date.to_iso_string().as_string().unwrap_or_default())
        }
    }
}

pub fn format_time_of_day(timestamp_ms: u64, locale: Locale) -> String {
    let date = js_sys::Date::new(&JsValue::from_f64(timestamp_ms as f64));
    date.to_locale_time_string(t::time::date_locale(locale))
        .as_string()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| date.to_iso_string().as_string().unwrap_or_default())
}
