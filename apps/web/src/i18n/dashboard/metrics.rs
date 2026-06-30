//! plan_ref:
//!   - 13_i18n#i18n-facade-contract
//!
//! Dashboard metric formatting facade.

use crate::i18n::Locale;

pub fn format_uptime(locale: Locale, secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    match locale {
        Locale::En if days > 0 => format!("{days}d {hours}h {minutes}m"),
        Locale::En if hours > 0 => format!("{hours}h {minutes}m"),
        Locale::En => format!("{minutes}m"),
        Locale::Zh if days > 0 => format!("{days} 天 {hours} 小时 {minutes} 分钟"),
        Locale::Zh if hours > 0 => format!("{hours} 小时 {minutes} 分钟"),
        Locale::Zh => format!("{minutes} 分钟"),
    }
}

pub fn format_bytes(locale: Locale, bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        match locale {
            Locale::En => format!("{bytes} B"),
            Locale::Zh => format!("{bytes} 字节"),
        }
    }
}

pub fn format_cpu_percent(_locale: Locale, value: f32) -> String {
    format!("{value:.1}%")
}

pub fn format_memory_mb(_locale: Locale, value: u64) -> String {
    format!("{value} MB")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_metric_formatting_is_localized() {
        assert_eq!(format_uptime(Locale::En, 90_060), "1d 1h 1m");
        assert_eq!(format_uptime(Locale::Zh, 90_060), "1 天 1 小时 1 分钟");
        assert_eq!(format_uptime(Locale::Zh, 3_600), "1 小时 0 分钟");
        assert_eq!(format_bytes(Locale::En, 1023), "1023 B");
        assert_eq!(format_bytes(Locale::Zh, 1023), "1023 字节");
        assert_eq!(format_bytes(Locale::Zh, 1_048_576), "1.0 MB");
        assert_eq!(format_cpu_percent(Locale::Zh, 12.34), "12.3%");
        assert_eq!(format_memory_mb(Locale::Zh, 42), "42 MB");
    }
}
