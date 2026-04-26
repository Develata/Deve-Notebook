// apps\web\src\i18n
//! # Extensions I18n

use super::Locale;

pub fn channel_desc(locale: Locale, mode: &str) -> &'static str {
    match (locale, mode) {
        (Locale::En, "agent-bridge") => {
            "Trusted external CLI bridge. Advanced mode and not enabled by default."
        }
        (Locale::Zh, "agent-bridge") => "受信任的外部 CLI 桥接。属于高级模式，默认不启用。",
        (Locale::En, _) => "Built-in native chat for lightweight markdown-first workflows.",
        (Locale::Zh, _) => "内置原生聊天，优先服务轻量、Markdown 优先的工作流。",
    }
}

pub fn runtime_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Plugin Runtime",
        Locale::Zh => "插件运行时",
    }
}

pub fn runtime_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Core notebook workflows ship first; installable plugin/runtime expansion stays optional."
        }
        Locale::Zh => "优先交付笔记本核心工作流；可安装插件与运行时扩展保持为可选项。",
    }
}

pub fn status_label(locale: Locale, active: bool) -> &'static str {
    match (locale, active) {
        (Locale::En, true) => "Active",
        (Locale::Zh, true) => "当前使用",
        (Locale::En, false) => "Switch",
        (Locale::Zh, false) => "切换",
    }
}

pub fn trusted_status_label(locale: Locale, active: bool, available: bool) -> &'static str {
    match (locale, active, available) {
        (Locale::En, _, false) => "Disabled",
        (Locale::Zh, _, false) => "禁用",
        _ => status_label(locale, active),
    }
}

pub fn trusted_cli_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Trusted CLI unavailable",
        Locale::Zh => "受信任 CLI 不可用",
    }
}

pub fn trusted_cli_fallback(locale: Locale, reason: &str) -> String {
    match locale {
        Locale::En => {
            format!("Trusted CLI is unavailable; switched back to Native AI. Reason: {reason}")
        }
        Locale::Zh => format!("受信任 CLI 不可用，已回退到原生 AI。原因：{reason}"),
    }
}

pub fn system_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "System Extensions",
        Locale::Zh => "系统扩展",
    }
}

pub fn bundled_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Bundled",
        Locale::Zh => "内置",
    }
}

pub fn planned_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Planned",
        Locale::Zh => "计划中",
    }
}

pub fn katex_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "KaTeX ships with the web shell for fast inline and block math rendering.",
        Locale::Zh => "KaTeX 已随 Web 壳内置，用于高性能行内与块级数学渲染。",
    }
}

pub fn mhchem_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Optional chemistry rendering planned behind config.tex_extensions = [\"mhchem\"]."
        }
        Locale::Zh => "化学公式扩展计划通过 config.tex_extensions = [\"mhchem\"] 按需启用。",
    }
}
