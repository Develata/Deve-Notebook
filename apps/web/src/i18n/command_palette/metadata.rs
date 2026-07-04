//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn group_navigation(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Navigation",
        Locale::Zh => "导航",
    }
}

pub fn group_settings(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Settings",
        Locale::Zh => "设置",
    }
}

pub fn group_layout(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Layout",
        Locale::Zh => "布局",
    }
}

pub fn group_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P / Branch",
        Locale::Zh => "P2P / 分支",
    }
}

pub fn group_source_control(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control",
        Locale::Zh => "源代码管理",
    }
}

pub fn group_git(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit Mirror",
        Locale::Zh => "ngit Mirror",
    }
}

pub fn group_remote_projection(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Projection",
        Locale::Zh => "远端 Projection",
    }
}

pub fn group_ai(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI",
        Locale::Zh => "AI",
    }
}

pub fn enabled_local_ui(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Available in the local UI shell",
        Locale::Zh => "在本地 UI Shell 中可用",
    }
}

pub fn enabled_local_settings(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Uses browser-local UI preferences and runtime feedback",
        Locale::Zh => "使用浏览器本地 UI 偏好与运行时反馈",
    }
}

pub fn enabled_search_surface(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Opens the current search surface",
        Locale::Zh => "打开当前搜索入口",
    }
}

pub fn enabled_peer_surface(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Requires an active peer or branch context",
        Locale::Zh => "需要当前节点或分支上下文",
    }
}

pub fn enabled_peer_merge_source(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Merges the selected read-only peer mirror into the local branch",
        Locale::Zh => "将选中的只读 peer mirror 合并到本地分支",
    }
}

pub fn enabled_cli_only_notice(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Shows a CLI-only notice; Web does not run the writer",
        Locale::Zh => "显示 CLI-only 提示；Web 不执行写命令",
    }
}

pub fn shortcut_ctrl_p() -> &'static str {
    "Ctrl+P"
}

pub fn shortcut_ctrl_shift_k() -> &'static str {
    "Ctrl+Shift+K"
}

pub fn shortcut_ctrl_l() -> &'static str {
    "Ctrl+L"
}

pub fn shortcut_ctrl_b() -> &'static str {
    "Ctrl+B"
}
