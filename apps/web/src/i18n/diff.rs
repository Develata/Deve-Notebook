//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!

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

pub fn loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading diff projection...",
        Locale::Zh => "正在加载对比投影...",
    }
}

pub fn waiting_for_draft(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Waiting to compute draft...",
        Locale::Zh => "等待计算草稿对比...",
    }
}

pub fn projection_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff projection unavailable",
        Locale::Zh => "对比投影不可用",
    }
}

pub fn retry(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Retry",
        Locale::Zh => "重试",
    }
}

pub fn split(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Split",
        Locale::Zh => "分栏",
    }
}

pub fn unified(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unified",
        Locale::Zh => "统一",
    }
}

pub fn invalid_projection(locale: Locale, detail: &str) -> String {
    match locale {
        Locale::En => format!("Invalid diff projection: {detail}"),
        Locale::Zh => format!("对比投影无效：{detail}"),
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

pub fn compute_ms(locale: Locale, ms: u32) -> String {
    match locale {
        Locale::En => format!("{} ms", ms),
        Locale::Zh => format!("{} 毫秒", ms),
    }
}

pub fn compute_ms_help(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Elapsed time for the latest backend diff computation; small files or cached projections can show 0 ms."
        }
        Locale::Zh => "最近一次后端 diff 计算耗时；小文件或缓存投影可能显示 0 毫秒。",
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
        Locale::En => {
            "Current line diff algorithm. Myers is the default; complex inputs may use Patience+Myers."
        }
        Locale::Zh => "当前行级 diff 算法。默认 Myers，复杂场景可能使用耐心法+Myers。",
    }
}

pub fn merge_conflict(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Merge conflict",
        Locale::Zh => "合并冲突",
    }
}

pub fn accept_current(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Accept Current",
        Locale::Zh => "接受当前",
    }
}

pub fn accept_incoming(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Accept Incoming",
        Locale::Zh => "接受传入",
    }
}

pub fn accept_result(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Accept Result",
        Locale::Zh => "接受结果",
    }
}
