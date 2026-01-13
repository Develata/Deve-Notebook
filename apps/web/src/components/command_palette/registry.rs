//! 命令面板的静态命令定义。

use leptos::prelude::*;
use crate::i18n::{Locale, t};
use super::types::Command;

/// 创建静态命令列表。
pub fn create_static_commands(
    locale: Locale,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
) -> Vec<Command> {
    vec![
        // 打开文档命令 - 打开文档模态框
        Command {
            id: "open".to_string(), 
            title: if locale == Locale::Zh { "打开文档".to_string() } else { "Open Document".to_string() },
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    on_open.run(());
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        Command {
            id: "settings".to_string(), 
            title: (t::command_palette::open_settings)(locale).to_string(),
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    on_settings.run(());
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        Command {
            id: "lang".to_string(),
            title: (t::command_palette::toggle_language)(locale).to_string(),
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    locale_signal.update(|l| *l = l.toggle());
                    set_show.set(false);
                });
            }),
            is_file: false,
        }
    ]
}

/// 基于查询字符串筛选命令。
pub fn filter_commands(
    query: &str,
    commands: Vec<Command>,
    max_results: usize,
) -> Vec<Command> {
    let q = query.to_lowercase();
    let mut results = Vec::new();
    
    for cmd in commands {
        if q.is_empty() || cmd.title.to_lowercase().contains(&q) || cmd.id.contains(&q) {
            results.push(cmd);
        }
    }
    
    if results.len() > max_results {
        results.truncate(max_results);
    }
    
    results
}
