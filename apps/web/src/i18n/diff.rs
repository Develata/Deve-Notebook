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

pub fn next_change(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Next change",
        Locale::Zh => "下一个变更",
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
