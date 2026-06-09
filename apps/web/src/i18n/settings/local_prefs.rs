//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 15_settings#browser-ui-prefs

use crate::i18n::Locale;

pub fn appearance(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Appearance",
        Locale::Zh => "外观",
    }
}

pub fn appearance_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Theme preference is stored locally in this browser.",
        Locale::Zh => "主题偏好只保存在当前浏览器本地。",
    }
}

pub fn theme_auto(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Auto",
        Locale::Zh => "自动",
    }
}

pub fn theme_light(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Light",
        Locale::Zh => "浅色",
    }
}

pub fn theme_dark(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Dark",
        Locale::Zh => "深色",
    }
}

pub fn editor_basics(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Editor",
        Locale::Zh => "编辑器",
    }
}

pub fn editor_basics_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Editor preferences are local UI markers for the current browser.",
        Locale::Zh => "编辑器偏好是当前浏览器内的本地 UI 标记。",
    }
}

pub fn word_wrap(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Word Wrap",
        Locale::Zh => "自动换行",
    }
}

pub fn word_wrap_on(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "On",
        Locale::Zh => "开启",
    }
}

pub fn word_wrap_off(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Off",
        Locale::Zh => "关闭",
    }
}

pub fn editor_density(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Density",
        Locale::Zh => "密度",
    }
}

pub fn editor_density_comfortable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Comfort",
        Locale::Zh => "舒适",
    }
}

pub fn editor_density_compact(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Compact",
        Locale::Zh => "紧凑",
    }
}

pub fn max_document_tabs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Max Document Tabs",
        Locale::Zh => "最大文档 Tab 数",
    }
}

pub fn max_document_tabs_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Applies only to Markdown document tabs.",
        Locale::Zh => "仅作用于 Markdown 文档 tab。",
    }
}

pub fn max_document_tabs_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Allowed range: 1 to 20.",
        Locale::Zh => "允许范围：1 到 20。",
    }
}

pub fn runtime_diagnostics(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Runtime Diagnostics",
        Locale::Zh => "运行诊断",
    }
}

pub fn runtime_diagnostics_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local smoke entry points for embedded and Trunk browser paths.",
        Locale::Zh => "嵌入式与 Trunk 浏览器路径的本地 smoke 入口。",
    }
}

pub fn embedded_runtime(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Embedded runtime",
        Locale::Zh => "嵌入式运行时",
    }
}

pub fn trunk_runtime(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Trunk fallback",
        Locale::Zh => "Trunk fallback",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Locale, appearance_desc, editor_basics_desc, max_document_tabs_desc,
        runtime_diagnostics_desc,
    };

    #[test]
    fn settings_v1_copy_marks_local_browser_boundaries_and_runtime_smoke() {
        assert!(appearance_desc(Locale::En).contains("locally"));
        assert!(editor_basics_desc(Locale::En).contains("local UI"));
        assert!(max_document_tabs_desc(Locale::En).contains("Markdown"));
        assert!(runtime_diagnostics_desc(Locale::En).contains("smoke"));
    }
}
