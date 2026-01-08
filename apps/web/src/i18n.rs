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
}
