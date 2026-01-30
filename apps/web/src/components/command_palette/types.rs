// apps\web\src\components\command_palette
//! 命令面板的命令类型定义。

#![allow(dead_code)] // is_file: 为文件搜索功能预留

use leptos::prelude::*;

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
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
