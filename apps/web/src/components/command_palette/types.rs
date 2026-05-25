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
            action,
            is_file: false,
            availability: CommandAvailability::Unavailable {
                reason: reason.into(),
            },
        }
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
