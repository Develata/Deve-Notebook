// apps\web\src\components\command_palette
//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!
//! 命令面板的命令类型定义。

#![allow(dead_code)] // is_file: 为文件搜索功能预留

use leptos::prelude::*;

/// Command Palette entry availability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandAvailability {
    Available,
    Unavailable { reason: String },
}

impl CommandAvailability {
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Unavailable { reason } => Some(reason.as_str()),
        }
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }
}

/// 可以从面板执行的命令。
#[derive(Clone, Debug)]
pub struct Command {
    /// 命令的唯一标识符。
    pub id: String,
    /// 面板中显示的标题。
    pub title: String,
    /// 用户可见的命令分组。
    pub group: String,
    /// 用户可见的快捷键提示。
    pub shortcut: Option<String>,
    /// 命令可用时的启用条件说明。
    pub enabled_when: String,
    /// 选中命令时执行的操作。
    pub action: Callback<()>,
    /// 该命令是否代表一个文件/文档。
    pub is_file: bool,
    /// 当前入口是否绑定可执行能力。
    pub availability: CommandAvailability,
}

impl Command {
    pub fn available(
        id: impl Into<String>,
        title: impl Into<String>,
        action: Callback<()>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group: "General".to_string(),
            shortcut: None,
            enabled_when: "Available".to_string(),
            action,
            is_file: false,
            availability: CommandAvailability::Available,
        }
    }

    pub fn unavailable(
        id: impl Into<String>,
        title: impl Into<String>,
        reason: impl Into<String>,
        action: Callback<()>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group: "General".to_string(),
            shortcut: None,
            enabled_when: "Unavailable".to_string(),
            action,
            is_file: false,
            availability: CommandAvailability::Unavailable {
                reason: reason.into(),
            },
        }
    }

    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_enabled_when(mut self, enabled_when: impl Into<String>) -> Self {
        self.enabled_when = enabled_when.into();
        self
    }

    pub fn detail_text(&self) -> String {
        self.availability
            .reason()
            .map(str::to_string)
            .unwrap_or_else(|| self.enabled_when.clone())
    }

    pub fn metadata_text(&self) -> String {
        match self.shortcut.as_deref() {
            Some(shortcut) if !shortcut.is_empty() => format!("{} · {shortcut}", self.group),
            _ => self.group.clone(),
        }
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
