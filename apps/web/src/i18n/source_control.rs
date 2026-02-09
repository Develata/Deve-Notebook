// apps\web\src\i18n
//! # I18n Source Control Module (源代码管理翻译)
//!
//! 包含版本控制面板相关的翻译字符串。

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control",
        Locale::Zh => "源代码管理",
    }
}

pub fn repositories(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Repositories",
        Locale::Zh => "存储库",
    }
}

pub fn changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Changes",
        Locale::Zh => "更改",
    }
}

pub fn staged_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Staged Changes",
        Locale::Zh => "暂存的更改",
    }
}

pub fn history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "History",
        Locale::Zh => "历史记录",
    }
}

pub fn graph(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph",
        Locale::Zh => "图形",
    }
}

pub fn open_file(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open File",
        Locale::Zh => "打开文件",
    }
}

pub fn stage_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage Changes",
        Locale::Zh => "暂存更改",
    }
}

pub fn unstage_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unstage Changes",
        Locale::Zh => "取消暂存更改",
    }
}

pub fn discard_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard Changes",
        Locale::Zh => "放弃更改",
    }
}

pub fn stage_all_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage All Changes",
        Locale::Zh => "暂存全部更改",
    }
}

pub fn unstage_all_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unstage All Changes",
        Locale::Zh => "取消暂存全部更改",
    }
}

pub fn discard_all_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard All Changes",
        Locale::Zh => "放弃全部更改",
    }
}

pub fn commit_message_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Message (Ctrl+Enter to commit on \"main\")",
        Locale::Zh => "提交信息（Ctrl+Enter 在“main”分支提交）",
    }
}

pub fn commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Commit",
        Locale::Zh => "提交",
    }
}

pub fn generate_commit_message(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Generate Commit Message",
        Locale::Zh => "生成提交信息",
    }
}

pub fn generate(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Generate",
        Locale::Zh => "生成",
    }
}

pub fn branch_main(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "main*",
        Locale::Zh => "主分支*",
    }
}

pub fn bulk_stage_progress(locale: Locale, done: u32, total: u32, failed: u32) -> String {
    match locale {
        Locale::En => format!("Staging {}/{} (failed: {})", done, total, failed),
        Locale::Zh => format!("暂存中 {}/{}（失败: {}）", done, total, failed),
    }
}

pub fn bulk_unstage_progress(locale: Locale, done: u32, total: u32, failed: u32) -> String {
    match locale {
        Locale::En => format!("Unstaging {}/{} (failed: {})", done, total, failed),
        Locale::Zh => format!("取消暂存中 {}/{}（失败: {}）", done, total, failed),
    }
}

pub fn bulk_done(locale: Locale, success: u32, total: u32) -> String {
    match locale {
        Locale::En => format!("Batch done: {}/{}", success, total),
        Locale::Zh => format!("批量完成：{}/{}", success, total),
    }
}

pub fn failed_paths(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Failed paths: {}", count),
        Locale::Zh => format!("失败文件：{}", count),
    }
}

pub fn show_more(locale: Locale, remain: usize) -> String {
    match locale {
        Locale::En => format!("Show more (+{})", remain),
        Locale::Zh => format!("显示更多（+{}）", remain),
    }
}
