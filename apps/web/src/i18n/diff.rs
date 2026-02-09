use super::Locale;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff",
        Locale::Zh => "对比",
    }
}

pub fn read_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Read Only",
        Locale::Zh => "只读",
    }
}

pub fn edit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Edit",
        Locale::Zh => "编辑",
    }
}

pub fn preview_diff(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Preview Diff",
        Locale::Zh => "预览对比",
    }
}

pub fn close_diff_view(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close Diff View",
        Locale::Zh => "关闭对比视图",
    }
}

pub fn computing(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Computing diff...",
        Locale::Zh => "正在计算对比...",
    }
}

pub fn prev_change(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Previous change",
        Locale::Zh => "上一个变更",
    }
}

pub fn prev_change_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Previous change (Shift+F7 / [)",
        Locale::Zh => "上一个变更（Shift+F7 / [）",
    }
}

pub fn next_change(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Next change",
        Locale::Zh => "下一个变更",
    }
}

pub fn next_change_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Next change (F7 / ])",
        Locale::Zh => "下一个变更（F7 / ]）",
    }
}

pub fn added(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Added lines",
        Locale::Zh => "新增行",
    }
}

pub fn deleted(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Deleted lines",
        Locale::Zh => "删除行",
    }
}

pub fn fold_unchanged(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Fold unchanged",
        Locale::Zh => "折叠未变更",
    }
}

pub fn show_all_lines(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Show all lines",
        Locale::Zh => "显示全部行",
    }
}

pub fn folded_lines(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("... {} unchanged lines (click to expand)", count),
        Locale::Zh => format!("... {} 行未变更（点击展开）", count),
    }
}

pub fn context_lines(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Context",
        Locale::Zh => "上下文",
    }
}

pub fn cache_hit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cache hit",
        Locale::Zh => "缓存命中",
    }
}

pub fn cache_miss(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cache miss",
        Locale::Zh => "缓存未命中",
    }
}

pub fn compute_ms(locale: Locale, ms: u32) -> String {
    match locale {
        Locale::En => format!("{} ms", ms),
        Locale::Zh => format!("{} 毫秒", ms),
    }
}

pub fn algorithm(locale: Locale, value: &str) -> String {
    let label = match value {
        "Patience+Myers" => match locale {
            Locale::En => "Patience+Myers",
            Locale::Zh => "耐心法+Myers",
        },
        _ => match locale {
            Locale::En => "Myers",
            Locale::Zh => "Myers",
        },
    };
    match locale {
        Locale::En => format!("Algo: {}", label),
        Locale::Zh => format!("算法: {}", label),
    }
}

pub fn algorithm_help(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Patience+Myers is preferred for stable hunk alignment.",
        Locale::Zh => "优先使用耐心法+Myers以获得更稳定的变更块对齐。",
    }
}

pub fn cache_ratio(locale: Locale, ratio: u32) -> String {
    match locale {
        Locale::En => format!("Hit: {}%", ratio),
        Locale::Zh => format!("命中率: {}%", ratio),
    }
}
