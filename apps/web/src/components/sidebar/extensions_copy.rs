use crate::i18n::Locale;

pub fn channel_desc(locale: Locale, mode: &str) -> &'static str {
    match (locale, mode) {
        (Locale::En, "agent-bridge") => "External CLI bridge with MCP, tools, and history.",
        (Locale::Zh, "agent-bridge") => "外部 CLI 桥接，支持 MCP、工具与历史能力。",
        (Locale::En, _) => "Built-in OpenAI-compatible chat for lightweight workflows.",
        (Locale::Zh, _) => "内置 OpenAI 兼容轻量聊天通道，适合轻量任务。",
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
        Locale::En => "Interface-first: bundled channels are ready, installable plugins come next.",
        Locale::Zh => "先完成接口层：内置通道已可用，可安装插件运行时仍在开发中。",
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
