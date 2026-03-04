//! # 时间格式化工具 (Time Formatting)
//!
//! 在 WASM 环境中使用 `js_sys::Date` 获取当前时间，
//! 将毫秒时间戳转换为相对时间显示。

/// 将毫秒时间戳转换为相对时间字符串
///
/// 例: "刚刚", "3 分钟前", "2 小时前", "昨天", "3 天前"
pub fn format_relative(timestamp_ms: i64) -> String {
    let now_ms = js_sys::Date::now() as i64;
    let diff_secs = (now_ms - timestamp_ms) / 1000;

    if diff_secs < 0 {
        return "刚刚".to_string();
    }

    let minutes = diff_secs / 60;
    let hours = diff_secs / 3600;
    let days = diff_secs / 86400;

    match diff_secs {
        0..=59 => "刚刚".to_string(),
        60..=3599 => format!("{} 分钟前", minutes),
        3600..=86399 => format!("{} 小时前", hours),
        86400..=172799 => "昨天".to_string(),
        172800..=604799 => format!("{} 天前", days),
        _ => {
            // 超过 7 天显示日期
            let date = js_sys::Date::new(&(timestamp_ms as f64).into());
            format!(
                "{}-{:02}-{:02}",
                date.get_full_year(),
                date.get_month() + 1,
                date.get_date()
            )
        }
    }
}
