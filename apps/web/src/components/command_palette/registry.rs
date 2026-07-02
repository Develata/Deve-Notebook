// apps\web\src\components\command_palette
//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 04_repository#repo-scope-runtime
//!
//! 命令面板的静态命令定义。

use super::types::Command;
use crate::components::main_layout::{ChatControl, SearchControl, SidebarControl};
use crate::hooks::use_core::{BranchContext, SourceControlContext, SyncMergeContext};
use crate::i18n::{Locale, persist_locale_preference, t};
use leptos::prelude::*;

mod branch;
mod git;
mod merge;
mod reserved;
#[cfg(test)]
mod tests;

use branch::establish_branch_command;
use git::{
    git_export_command, git_import_command, git_mirror_command, git_push_command,
    git_repair_command, git_status_command,
};
use merge::merge_peer_commands;
use reserved::{ai_reserved_commands, source_control_reserved_commands};

#[derive(Clone, Default)]
pub struct StaticCommandContext {
    chat_control: Option<ChatControl>,
    search_control: Option<SearchControl>,
    sidebar_control: Option<SidebarControl>,
    branch_context: Option<BranchContext>,
    source_control_context: Option<SourceControlContext>,
    sync_merge_context: Option<SyncMergeContext>,
}

impl StaticCommandContext {
    pub fn from_current_context() -> Self {
        Self {
            chat_control: use_context::<ChatControl>(),
            search_control: use_context::<SearchControl>(),
            sidebar_control: use_context::<SidebarControl>(),
            branch_context: use_context::<BranchContext>(),
            source_control_context: use_context::<SourceControlContext>(),
            sync_merge_context: use_context::<SyncMergeContext>(),
        }
    }

    pub fn with_source_control_context(mut self, source_control: SourceControlContext) -> Self {
        self.source_control_context = Some(source_control);
        self
    }
}

/// 创建静态命令列表。
pub fn create_static_commands(
    locale: Locale,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
) -> Vec<Command> {
    create_static_commands_with_context(
        locale,
        on_settings,
        on_open,
        set_show,
        locale_signal,
        StaticCommandContext::from_current_context(),
    )
}

pub fn create_static_commands_with_context(
    locale: Locale,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
    context: StaticCommandContext,
) -> Vec<Command> {
    let StaticCommandContext {
        chat_control,
        search_control,
        sidebar_control,
        branch_context,
        source_control_context,
        sync_merge_context,
    } = context;

    let mut commands = vec![
        // 打开文档命令 - 打开文档模态框
        Command::available(
            "open",
            (t::command_palette::open_document)(locale),
            Callback::new(move |_| {
                on_open.run(());
                // Do not close, as on_open re-purposes the search box
            }),
        )
        .with_group((t::command_palette::group_navigation)(locale))
        .with_shortcut(t::command_palette::shortcut_ctrl_p())
        .with_enabled_when((t::command_palette::enabled_search_surface)(locale)),
        Command::available(
            "settings",
            (t::command_palette::open_settings)(locale),
            Callback::new(move |_| {
                on_settings.run(());
                set_show.set(false);
            }),
        )
        .with_group((t::command_palette::group_settings)(locale))
        .with_enabled_when((t::command_palette::enabled_local_settings)(locale)),
        Command::available(
            "lang",
            (t::command_palette::toggle_language)(locale),
            Callback::new(move |_| {
                locale_signal.update(|locale| {
                    *locale = locale.toggle();
                    persist_locale_preference(*locale);
                });
                set_show.set(false);
            }),
        )
        .with_group((t::command_palette::group_settings)(locale))
        .with_shortcut(t::command_palette::shortcut_ctrl_l())
        .with_enabled_when((t::command_palette::enabled_local_settings)(locale)),
        // P2P: Switch to Peer
        Command::available(
            "switch_peer",
            (t::command_palette::switch_peer)(locale),
            Callback::new(move |_| {
                if let Some(search_control) = search_control {
                    search_control.set_mode.set("@".to_string());
                    search_control.set_show.set(true);
                }
                set_show.set(false);
            }),
        )
        .with_group((t::command_palette::group_peer)(locale))
        .with_shortcut(t::command_palette::shortcut_ctrl_shift_k())
        .with_enabled_when((t::command_palette::enabled_peer_surface)(locale)),
    ];

    if let Some(sidebar_control) = sidebar_control {
        commands.push(
            Command::available(
                "toggle_sidebar",
                (t::command_palette::toggle_sidebar)(locale),
                Callback::new(move |_| {
                    sidebar_control
                        .set_visible
                        .update(|visible| *visible = !*visible);
                    set_show.set(false);
                }),
            )
            .with_group((t::command_palette::group_layout)(locale))
            .with_shortcut(t::command_palette::shortcut_ctrl_b())
            .with_enabled_when((t::command_palette::enabled_local_settings)(locale)),
        );
    }

    commands.extend(source_control_reserved_commands(locale));
    commands.push(establish_branch_command(
        locale,
        set_show,
        source_control_context.clone(),
    ));
    commands.extend(merge_peer_commands(
        locale,
        set_show,
        branch_context,
        sync_merge_context,
    ));
    commands.extend(vec![
        git_status_command(locale, set_show, source_control_context.clone()),
        git_mirror_command(locale, set_show, source_control_context.clone()),
        git_export_command(locale, set_show, source_control_context.clone()),
        git_import_command(locale, set_show, source_control_context.clone()),
        git_push_command(locale, set_show, source_control_context.clone()),
        git_repair_command(locale, set_show, source_control_context),
    ]);

    // Add AI Chat toggle command if ChatControl is available
    if let Some(chat_ctrl) = chat_control {
        commands.push(
            Command::available(
                "toggle_ai_chat",
                (t::command_palette::toggle_ai_chat)(locale),
                Callback::new(move |_| {
                    let current = chat_ctrl.chat_visible.get_untracked();
                    chat_ctrl.set_chat_visible.set(!current);
                    set_show.set(false);
                }),
            )
            .with_group((t::command_palette::group_ai)(locale))
            .with_enabled_when((t::command_palette::enabled_local_ui)(locale)),
        );
    }
    commands.extend(ai_reserved_commands(locale));

    commands
}

/// 基于查询字符串筛选命令。
pub fn filter_commands(query: &str, commands: Vec<Command>, max_results: usize) -> Vec<Command> {
    let q = query.to_lowercase();
    let mut results = Vec::new();

    for cmd in commands {
        let shortcut = cmd.shortcut.as_deref().unwrap_or_default().to_lowercase();
        if q.is_empty()
            || cmd.title.to_lowercase().contains(&q)
            || cmd.id.contains(&q)
            || cmd.group.to_lowercase().contains(&q)
            || shortcut.contains(&q)
            || cmd.enabled_when.to_lowercase().contains(&q)
        {
            results.push(cmd);
        }
    }

    if results.len() > max_results {
        results.truncate(max_results);
    }

    results
}
