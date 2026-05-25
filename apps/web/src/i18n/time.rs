//! Time and relative-time strings.
//! plan_ref:
//!   - 13_i18n#i18n-facade-contract

use super::Locale;

pub fn just_now(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "just now",
        Locale::Zh => "刚刚",
    }
}

pub fn yesterday(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "yesterday",
        Locale::Zh => "昨天",
    }
}

pub fn minutes_ago(locale: Locale, minutes: i64) -> String {
    match locale {
        Locale::En => plural(minutes, "minute"),
        Locale::Zh => format!("{minutes} 分钟前"),
    }
}

pub fn hours_ago(locale: Locale, hours: i64) -> String {
    match locale {
        Locale::En => plural(hours, "hour"),
        Locale::Zh => format!("{hours} 小时前"),
    }
}

pub fn days_ago(locale: Locale, days: i64) -> String {
    match locale {
        Locale::En => plural(days, "day"),
        Locale::Zh => format!("{days} 天前"),
    }
}

pub fn date_locale(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "en-US",
        Locale::Zh => "zh-CN",
    }
}

fn plural(value: i64, unit: &str) -> String {
    let suffix = if value == 1 { "" } else { "s" };
    format!("{value} {unit}{suffix} ago")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_copy_is_localized() {
        assert_eq!(just_now(Locale::En), "just now");
        assert_eq!(just_now(Locale::Zh), "刚刚");
        assert_eq!(minutes_ago(Locale::En, 2), "2 minutes ago");
        assert_eq!(minutes_ago(Locale::Zh, 2), "2 分钟前");
    }
}
