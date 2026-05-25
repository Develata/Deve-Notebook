//! # 时间格式化工具 (Time Formatting)
//! plan_ref:
//!   - 13_i18n#i18n-facade-contract
//!
//! 在 WASM 环境中使用 `js_sys::Date` 获取当前时间，
//! 将毫秒时间戳转换为相对时间显示。

use crate::i18n::{Locale, time as time_i18n};
use wasm_bindgen::JsValue;

/// 将毫秒时间戳转换为相对时间字符串
///
/// 例: "刚刚", "3 分钟前", "2 小时前", "昨天", "3 天前"
pub fn format_relative(timestamp_ms: i64, locale: Locale) -> String {
    let now_ms = js_sys::Date::now() as i64;
    let diff_secs = (now_ms - timestamp_ms) / 1000;

    if diff_secs < 0 {
        return time_i18n::just_now(locale).to_string();
    }

    let minutes = diff_secs / 60;
    let hours = diff_secs / 3600;
    let days = diff_secs / 86400;

    match diff_secs {
        0..=59 => time_i18n::just_now(locale).to_string(),
        60..=3599 => time_i18n::minutes_ago(locale, minutes),
        3600..=86399 => time_i18n::hours_ago(locale, hours),
        86400..=172799 => time_i18n::yesterday(locale).to_string(),
        172800..=604799 => time_i18n::days_ago(locale, days),
        _ => {
            let date = js_sys::Date::new(&(timestamp_ms as f64).into());
            date.to_locale_date_string(time_i18n::date_locale(locale), &JsValue::UNDEFINED)
                .as_string()
                .unwrap_or_else(|| date.to_iso_string().as_string().unwrap_or_default())
        }
    }
}

pub fn format_time_of_day(timestamp_ms: u64, locale: Locale) -> String {
    let date = js_sys::Date::new(&JsValue::from_f64(timestamp_ms as f64));
    date.to_locale_time_string(time_i18n::date_locale(locale))
        .as_string()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| date.to_iso_string().as_string().unwrap_or_default())
}
