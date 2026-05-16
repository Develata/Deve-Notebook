// apps\web\src\i18n
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!
//! # I18n Command Palette Module (命令面板翻译)

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

pub fn placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Type a command...",
        Locale::Zh => "输入命令...",
    }
}

pub fn no_results(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No results found.",
        Locale::Zh => "未找到结果。",
    }
}

pub fn open_settings(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Settings (config)",
        Locale::Zh => "打开设置 (config)",
    }
}

pub fn toggle_language(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle Language",
        Locale::Zh => "切换语言",
    }
}

pub fn open_document(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Document",
        Locale::Zh => "打开文档",
    }
}

pub fn switch_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Switch to Peer",
        Locale::Zh => "P2P: 切换到节点",
    }
}

pub fn establish_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Establish Branch",
        Locale::Zh => "P2P: 建立分支",
    }
}

pub fn establish_branch_unavailable_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: no branch creation backend",
        Locale::Zh => "不可用：尚无分支创建后端",
    }
}

pub fn merge_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Merge Peer",
        Locale::Zh => "P2P: 合并当前节点",
    }
}

pub fn source_control_sync(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control: Sync",
        Locale::Zh => "Source Control: 同步",
    }
}

pub fn source_control_commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control: Commit",
        Locale::Zh => "Source Control: 提交",
    }
}

pub fn source_control_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control: Push",
        Locale::Zh => "Source Control: 推送",
    }
}

pub fn source_control_panel_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Unavailable: use the Source Control panel for scoped state and message input"
        }
        Locale::Zh => "不可用：请在 Source Control 面板中使用带作用域与提交信息的操作",
    }
}

pub fn git_status(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Status",
        Locale::Zh => "Git: 状态",
    }
}

pub fn git_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Mirror",
        Locale::Zh => "Git: 执行 Mirror",
    }
}

pub fn git_export_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Export Mirror",
        Locale::Zh => "Git: 导出 Mirror",
    }
}

pub fn git_cli_only_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "CLI-only: Web does not execute Git writer commands",
        Locale::Zh => "CLI-only：Web 不执行 Git 写命令",
    }
}

pub fn git_import_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Import Changes",
        Locale::Zh => "Git: 导入外部变更",
    }
}

pub fn git_push_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Push Mirror",
        Locale::Zh => "Git: 推送 Mirror",
    }
}

pub fn git_repair_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Repair Mirror",
        Locale::Zh => "Git: 修复 Mirror",
    }
}

pub fn toggle_ai_chat(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Toggle Chat Panel",
        Locale::Zh => "AI: 切换聊天面板",
    }
}

pub fn ai_retry_last_request(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Retry Last Request",
        Locale::Zh => "AI: 重试上次请求",
    }
}

pub fn ai_switch_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Switch Backend",
        Locale::Zh => "AI: 切换后端",
    }
}

pub fn ai_switch_plan(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Switch to PLAN Mode",
        Locale::Zh => "AI: 切换到 PLAN 模式",
    }
}

pub fn ai_switch_build(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Switch to BUILD Mode",
        Locale::Zh => "AI: 切换到 BUILD 模式",
    }
}

pub fn ai_retry_panel_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: retry is scoped to the open AI Chat panel",
        Locale::Zh => "不可用：重试只在已打开的 AI Chat 面板内生效",
    }
}

pub fn ai_backend_settings_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: backend switching is gated by Settings and server capabilities",
        Locale::Zh => "不可用：后端切换由 Settings 与服务端能力门禁控制",
    }
}

pub fn ai_slash_mode_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: use /plan or /build in AI Chat",
        Locale::Zh => "不可用：请在 AI Chat 中使用 /plan 或 /build",
    }
}

pub fn keyboard_navigate_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "to navigate",
        Locale::Zh => "导航",
    }
}

pub fn keyboard_select_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "to select",
        Locale::Zh => "选择",
    }
}

pub fn keyboard_close_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "to close",
        Locale::Zh => "关闭",
    }
}

#[cfg(test)]
mod tests;
