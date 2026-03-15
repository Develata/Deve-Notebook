// apps\web\src\components\sidebar\source_control
//! # ChangeItem 组件 (变更条目组件)
//!
//! 渲染单个文件变更条目，包含文件图标、名称、路径和状态标记。
//! 支持 Stage/Unstage/Open/Discard 操作。

use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::{ChangeEntry, ChangeStatus, ConflictResolution};
use leptos::prelude::*;

/// 变更条目组件
///
/// # 参数
/// - `entry`: 变更条目数据
/// - `is_staged`: 是否为暂存区条目
#[component]
pub fn ChangeItem(entry: ChangeEntry, is_staged: bool) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let current_repo_id = core.current_repo_id;
    let pending_branch_switch = core.pending_branch_switch;
    let pending_repo_switch = core.pending_repo_switch;

    let has_conflict = entry.has_conflict;
    let full_path = entry.path.clone();
    let renamed_from = entry.renamed_from.clone();
    let path_parts: Vec<&str> = full_path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();
    let display_name = renamed_from
        .as_ref()
        .and_then(|old_path| old_path.rsplit('/').next())
        .map(|old_name| format!("{} -> {}", old_name, filename))
        .unwrap_or_else(|| filename.clone());

    // 目录路径 (不含文件名)
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len() - 1].join("/")
    } else {
        String::new()
    };

    let entry_for_click = entry.clone();
    let entry_for_unstage = entry.clone();
    let entry_for_keep_fs = entry.clone();
    let entry_for_keep_ledger = entry.clone();
    let entry_for_open = entry.clone();
    let entry_for_discard = entry.clone();
    let entry_for_stage = entry.clone();

    // 状态图标和颜色
    let (icon_char, color_cls) = match entry.status {
        ChangeStatus::Modified => ("M", "text-modified"),
        ChangeStatus::Added if renamed_from.is_some() => ("R", "text-added"),
        ChangeStatus::Added => ("A", "text-added"),
        ChangeStatus::Deleted => ("D", "text-deleted"),
        ChangeStatus::Renamed => ("R", "text-added"),
    };

    view! {
        <div
            class=format!("flex items-center px-4 py-0.5 hover:bg-hover text-[13px] group cursor-pointer h-[22px] {}", if has_conflict { "text-warning bg-warning/5" } else { "text-primary" })
            on:click=move |_| {
                if current_repo_id.get().is_none()
                    || pending_branch_switch.get().is_some()
                    || pending_repo_switch.get().is_some()
                {
                    return;
                }
                // 点击任何条目都打开 diff 视图 (与 VS Code 行为一致)
                core.on_get_doc_diff.run(entry_for_click.clone());
            }
        >
            <div class="flex items-center gap-1.5 flex-1 overflow-hidden">
                // 文件图标
                <FileText class=format!("w-3.5 h-3.5 min-w-3.5 {}", if filename.ends_with(".rs") { "text-[var(--color-file-rust)]" } else { "text-muted" }) />

                <span class="truncate">{display_name}</span>
                <span class="text-xs text-muted truncate shrink-0 ml-1">
                    {directory}
                </span>
            </div>

            <div class="flex items-center gap-2 pl-2">
                // 操作按钮 (悬停显示)
                <div class="hidden group-hover:!flex items-center gap-0.5 mr-1">
                    {if is_staged {
                        // 暂存区: 仅显示 Unstage 按钮
                        view! {
                                <button
                                    class="p-0.5 hover:bg-active rounded text-secondary"
                                    disabled=move || {
                                        current_repo_id.get().is_none()
                                            || pending_branch_switch.get().is_some()
                                            || pending_repo_switch.get().is_some()
                                    }
                                    title=move || t::source_control::unstage_changes(locale.get())
                                    on:click=move |ev| { ev.stop_propagation(); core.on_unstage_file.run(entry_for_unstage.clone()); }
                                >
                                <Minus class="w-3.5 h-3.5" />
                            </button>
                        }.into_any()
                    } else if has_conflict {
                        // 冲突状态: Keep FS / Keep Ledger / Stage
                        view! {
                            <button
                                class="p-0.5 hover:bg-active rounded text-warning"
                                disabled=move || {
                                    current_repo_id.get().is_none()
                                        || pending_branch_switch.get().is_some()
                                        || pending_repo_switch.get().is_some()
                                }
                                title=move || t::source_control::keep_file_system(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_resolve_conflict.run((entry_for_keep_fs.clone(), ConflictResolution::KeepFs)); }
                            >
                                <Upload class="w-3.5 h-3.5" />
                            </button>
                            <button
                                class="p-0.5 hover:bg-active rounded text-warning"
                                disabled=move || {
                                    current_repo_id.get().is_none()
                                        || pending_branch_switch.get().is_some()
                                        || pending_repo_switch.get().is_some()
                                }
                                title=move || t::source_control::keep_ledger(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_resolve_conflict.run((entry_for_keep_ledger.clone(), ConflictResolution::KeepLedger)); }
                            >
                                <Download class="w-3.5 h-3.5" />
                            </button>
                        }.into_any()
                    } else {
                        // 工作区: Open, Discard, Stage
                        view! {
                            <button
                                class="p-0.5 hover:bg-active rounded text-secondary"
                                disabled=move || {
                                    current_repo_id.get().is_none()
                                        || pending_branch_switch.get().is_some()
                                        || pending_repo_switch.get().is_some()
                                }
                                title=move || t::source_control::open_file(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_get_doc_diff.run(entry_for_open.clone()); }
                            >
                                <ExternalLink class="w-3.5 h-3.5" />
                            </button>
                            <button
                                class="p-0.5 hover:bg-active rounded text-secondary"
                                disabled=move || {
                                    current_repo_id.get().is_none()
                                        || pending_branch_switch.get().is_some()
                                        || pending_repo_switch.get().is_some()
                                }
                                title=move || t::source_control::discard_changes(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_discard_file.run(entry_for_discard.clone()); }
                            >
                                <RotateCcw class="w-3.5 h-3.5" />
                            </button>
                            <button
                                class="p-0.5 hover:bg-active rounded text-secondary"
                                disabled=move || {
                                    current_repo_id.get().is_none()
                                        || pending_branch_switch.get().is_some()
                                        || pending_repo_switch.get().is_some()
                                }
                                title=move || t::source_control::stage_changes(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_stage_file.run(entry_for_stage.clone()); }
                            >
                                <Plus class="w-3.5 h-3.5" />
                            </button>
                        }.into_any()
                    }}
                </div>

                // 冲突指示 + 状态标记
                {if has_conflict {
                    view! { <AlertTriangle class="w-3 h-3 text-warning mr-0.5" /> }.into_any()
                } else {
                    view! {}.into_any()
                }}
                <span class=format!("{} text-[11px] font-bold w-3 text-center", color_cls)>
                    {icon_char}
                </span>
            </div>
        </div>
    }
}
