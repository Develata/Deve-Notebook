// apps\web\src\components\source_control
//! # History Component (历史记录组件)
//!
//! VS Code 风格: Timeline 视图。
//! 左侧带有连接线和圆点，点击提交可展开文件差异列表。
use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use crate::utils::time::format_relative;
use deve_core::source_control::ChangeStatus;
use leptos::prelude::*;

#[component]
pub fn History(expanded: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    // 当前展开的提交 ID (点击切换)
    let selected_commit = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        if core.current_repo_id.get().is_none()
            || core.pending_branch_switch.get().is_some()
            || core.pending_repo_switch.get().is_some()
        {
            return;
        }
        let _ = core.active_branch.get();
        core.on_get_history.run(20);
    });

    view! {
        <div class="border-t border-default">
            <button
                class="w-full flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase"
                on:click=move |_| expanded.update(|b| *b = !*b)
            >
                <span class=move || if expanded.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                    <ChevronRight class="w-3 h-3" />
                </span>
                {move || t::source_control::graph(locale.get())}
            </button>

            {move || if expanded.get() {
                view! {
                    <div class="pb-2">
                        // Timeline List
                        <div class="relative pl-6 pt-2">
                            // Vertical Line
                            <div class="absolute left-[19px] top-2 bottom-0 w-[1px] bg-active"></div>

                            <For
                                each=move || core.commit_history.get()
                                key=|c| c.id.clone()
                                children=move |commit| {
                                    view! {
                                        <div class="relative mb-3 group">
                                            // Dot
                                            <div class="absolute -left-[19px] top-[3px] w-2.5 h-2.5 rounded-full border-2 border-white bg-accent shadow-sm z-10"></div>

                                            <div
                                                class="pr-2 cursor-pointer"
                                                on:click={
                                                    let cid = commit.id.clone();
                                                    let pid = commit.parent_id.clone();
                                                    move |_| {
                                                        if selected_commit.get().as_deref() == Some(&cid) {
                                                            selected_commit.set(None);
                                                        } else {
                                                            core.on_get_commit_diff.run((pid.clone(), cid.clone()));
                                                            selected_commit.set(Some(cid.clone()));
                                                        }
                                                    }
                                                }
                                            >
                                                <div class="text-[13px] text-primary leading-tight mb-0.5 font-medium truncate" title={commit.message.clone()}>
                                                    {commit.message.clone()}
                                                </div>
                                                <div class="flex items-center gap-2 text-[11px] text-muted">
                                                    <span class="font-mono bg-hover px-1 rounded text-secondary">{commit.id[0..7].to_string()}</span>
                                                    <span>{format_relative(commit.timestamp)}</span>
                                                </div>
                                            </div>

                                            // 内联展开: 提交的文件差异列表
                                            {
                                                let cid = commit.id.clone();
                                                move || {
                                                    if selected_commit.get().as_deref() == Some(&cid) {
                                                        let diffs = core.commit_diff_result.get();
                                                        view! {
                                                            <div class="ml-2 mt-1 border-l border-active pl-2">
                                                                {diffs.into_iter().map(|f| {
                                                                    let path_label = f.previous_path
                                                                        .as_ref()
                                                                        .map(|old| format!("{old} -> {}", f.path))
                                                                        .unwrap_or_else(|| f.path.clone());
                                                                    let (ch, cls) = match f.status {
                                                                        ChangeStatus::Modified => ("M", "text-modified"),
                                                                        ChangeStatus::Added => ("A", "text-added"),
                                                                        ChangeStatus::Deleted => ("D", "text-deleted"),
                                                                        ChangeStatus::Renamed => ("R", "text-added"),
                                                                    };
                                                                    view! {
                                                                        <div class="flex items-center gap-1 text-[12px] text-secondary py-0.5 hover:bg-hover px-1 rounded cursor-pointer">
                                                                            <FileText class="w-3 h-3 text-muted" />
                                                                            <span class="truncate flex-1">{path_label}</span>
                                                                            <span class=format!("{cls} text-[10px] font-bold")>{ch}</span>
                                                                        </div>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! {}.into_any()
                                                    }
                                                }
                                            }
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
