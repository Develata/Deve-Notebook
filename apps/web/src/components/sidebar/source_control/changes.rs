// apps\web\src\components\sidebar\source_control
//! # Changes 组件 (变更列表组件)
//!
//! 组合 `StagedSection` 和 `UnstagedSection` 子组件，
//! 显示完整的变更列表视图。

use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::staged_section::StagedSection;
use super::unstaged_section::UnstagedSection;

/// 变更列表主组件
///
/// 职责:
/// - 触发变更数据获取
/// - 分发数据到子组件
#[component]
pub fn Changes() -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // 初次加载时获取变更数据
    Effect::new(move |_| {
        core.on_get_changes.run(());
    });

    view! {
        <div>
            <Show when=move || core.sc_bulk_progress.get().is_some()>
                <div class="mx-2 my-1 px-2 py-1 rounded border border-blue-200 bg-blue-50 text-[11px] text-blue-700">
                    {move || {
                        core.sc_bulk_progress
                            .get()
                            .map(|p| {
                                if p.op == "unstage" {
                                    t::source_control::bulk_unstage_progress(
                                        locale.get(),
                                        p.done,
                                        p.total,
                                        p.failed,
                                    )
                                } else {
                                    t::source_control::bulk_stage_progress(
                                        locale.get(),
                                        p.done,
                                        p.total,
                                        p.failed,
                                    )
                                }
                            })
                            .unwrap_or_default()
                    }}
                </div>
            </Show>
            <Show when=move || !core.sc_bulk_failed_paths.get().is_empty()>
                <div class="mx-2 my-1 px-2 py-1 rounded border border-amber-200 bg-amber-50 text-[11px] text-amber-700">
                    {move || t::source_control::failed_paths(locale.get(), core.sc_bulk_failed_paths.get().len())}
                </div>
            </Show>
            {move || {
                let staged = core.staged_changes.get();
                let unstaged = core.unstaged_changes.get();

                view! {
                    <div>
                        <StagedSection staged=staged />
                        <UnstagedSection unstaged=unstaged />
                    </div>
                }
            }}
        </div>
    }
}
