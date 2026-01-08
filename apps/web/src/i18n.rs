use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Locale {
    #[default]
    En,
    Zh,
}

impl Locale {
    pub const fn toggle(&self) -> Self {
        match self {
            Self::En => Self::Zh,
            Self::Zh => Self::En,
        }
    }
}

// Translation Modules
pub mod t {
    use super::Locale;

    pub fn app_title(locale: Locale) -> &'static str {
        match locale {
            Locale::En => "Deve-Note",
            Locale::Zh => "Deve-Note",
        }
    }

    pub mod header {
        use super::Locale;
        pub fn settings(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Settings",
                Locale::Zh => "设置",
            }
        }
        pub fn home(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Home",
                Locale::Zh => "首页",
            }
        }
        pub fn open(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Open Index",
                Locale::Zh => "打开目录",
            }
        }
        pub fn command(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Command Palette",
                Locale::Zh => "命令面板",
            }
        }
    }

    pub mod sidebar {
        use super::Locale;
        pub fn no_docs(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "No documents found",
                Locale::Zh => "暂无文档",
            }
        }
    }

    pub mod settings {
        use super::Locale;
        
        pub fn title(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Settings",
                Locale::Zh => "设置",
            }
        }
        pub fn close(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Close",
                Locale::Zh => "关闭",
            }
        }
        pub fn about(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "About",
                Locale::Zh => "关于",
            }
        }
        pub fn version(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Version",
                Locale::Zh => "版本",
            }
        }
        pub fn language(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Language",
                Locale::Zh => "语言",
            }
        }
        pub fn hybrid_mode(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Hybrid Editing",
                Locale::Zh => "混合编辑",
            }
        }
        pub fn hybrid_desc(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Hide Markdown syntax while reading",
                Locale::Zh => "阅读时隐藏 Markdown 语法",
            }
        }
        pub fn coming_soon(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Coming in Phase 6",
                Locale::Zh => "将在 Phase 6 推出",
            }
        }
    }

    pub mod bottom_bar {
        use super::Locale;
        
        pub fn words(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Words",
                Locale::Zh => "字数",
            }
        }
        pub fn lines(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Lines",
                Locale::Zh => "行数",
            }
        }
        pub fn col(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Col",
                Locale::Zh => "列",
            }
        }
        pub fn ready(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Ready",
                Locale::Zh => "就绪",
            }
        }
        pub fn syncing(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Syncing...",
                Locale::Zh => "同步中...",
            }
        }
        pub fn offline(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "Offline",
                Locale::Zh => "离线",
            }
        }
    }

    pub mod playback {
        use super::Locale;
        pub fn label(locale: Locale) -> &'static str {
            match locale {
                Locale::En => "PLAYBACK",
                Locale::Zh => "回放控制",
            }
        }
    }

    pub mod command_palette {
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
    }

}
