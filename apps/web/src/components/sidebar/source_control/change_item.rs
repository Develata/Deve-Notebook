// apps\web\src\components\sidebar\source_control
//! # ChangeItem 组件 (变更条目组件)
//!
//! 渲染单个文件变更条目，包含文件图标、名称、路径和状态标记。
//! 支持 Stage/Unstage/Open/Discard 操作。

use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
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

    let full_path = entry.path.clone();
    let path_parts: Vec<&str> = full_path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();

    // 目录路径 (不含文件名)
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len() - 1].join("/")
    } else {
        String::new()
    };

    let path_for_stage = full_path.clone();
    let path_for_unstage = full_path.clone();
    let path_for_open = full_path.clone();
    let path_for_discard = full_path.clone();

    // 状态图标和颜色
    let (icon_char, color_cls) = match entry.status {
        ChangeStatus::Modified => ("M", "text-modified"),
        ChangeStatus::Added => ("A", "text-added"),
        ChangeStatus::Deleted => ("D", "text-deleted"),
    };

    view! {
        <div
            class="flex items-center px-4 py-0.5 hover:bg-hover text-[13px] group cursor-pointer h-[22px] text-primary"
            on:click=move |_| {
                // 点击任何条目都打开 diff 视图 (与 VS Code 行为一致)
                core.on_get_doc_diff.run(full_path.clone());
            }
        >
            <div class="flex items-center gap-1.5 flex-1 overflow-hidden">
                // 文件图标
                <FileText class=format!("w-3.5 h-3.5 min-w-3.5 {}", if filename.ends_with(".rs") { "text-[#dea584]" } else { "text-muted" }) />

                <span class="truncate">{filename}</span>
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
                                    title=move || t::source_control::unstage_changes(locale.get())
                                    on:click=move |ev| { ev.stop_propagation(); core.on_unstage_file.run(path_for_unstage.clone()); }
                                >
                                <Minus class="w-3.5 h-3.5" />
                            </button>
                        }.into_any()
                    } else {
                        // 工作区: Open, Discard, Stage
                        view! {
                            <button
                                class="p-0.5 hover:bg-active rounded text-secondary"
                                title=move || t::source_control::open_file(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_get_doc_diff.run(path_for_open.clone()); }
                            >
                                <ExternalLink class="w-3.5 h-3.5" />
                            </button>
                            <button
                                class="p-0.5 hover:bg-active rounded text-secondary"
                                title=move || t::source_control::discard_changes(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_discard_file.run(path_for_discard.clone()); }
                            >
                                <RotateCcw class="w-3.5 h-3.5" />
                            </button>
                            <button
                                class="p-0.5 hover:bg-active rounded text-secondary"
                                title=move || t::source_control::stage_changes(locale.get())
                                on:click=move |ev| { ev.stop_propagation(); core.on_stage_file.run(path_for_stage.clone()); }
                            >
                                <Plus class="w-3.5 h-3.5" />
                            </button>
                        }.into_any()
                    }}
                </div>

                // 状态标记 (M/A/D)
                <span class=format!("{} text-[11px] font-bold w-3 text-center", color_cls)>
                    {icon_char}
                </span>
            </div>
        </div>
    }
}
