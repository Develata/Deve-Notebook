// apps\web\src\components\sidebar\source_control
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # StagedSection 组件 (暂存区组件)
//!
//! 渲染暂存区 (Staged Changes) 的文件列表。

use super::change_item::ChangeItem;
use super::staged_section_actions::StagedSectionActions;
use super::touch_target::section_header_class;
use crate::components::icons::ChevronRight;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

/// 暂存区组件
#[component]
pub fn StagedSection(staged: Vec<ChangeEntry>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (bulk_busy, set_bulk_busy) = signal(false);
    let expanded = RwSignal::new(true);

    let staged_count = staged.len();
    let staged_list = StoredValue::new(staged.clone());
    let staged_list_for_action = StoredValue::new(staged);

    Effect::new(move |_| {
        let _ = core.unstaged_changes.get();
        let _ = core.staged_changes.get();
        let _ = core.confirmed_changes.get();
        set_bulk_busy.set(false);
    });

    if staged_count == 0 {
        return view! {}.into_any();
    }

    view! {
        <div data-deve-source-control-section="staged">
            <div
                class=section_header_class()
                data-deve-mobile-touch-target="source-control-section-header"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <div class="flex items-center">
                    <span class=move || format!("w-4 h-4 flex items-center justify-center text-primary transition-transform {}", if expanded.get() { "rotate-90" } else { "" })>
                        <ChevronRight class="w-3 h-3" />
                    </span>
                    <span class="text-[11px] font-bold text-primary uppercase">
                        {move || t::source_control::staged_changes(locale.get())}
                    </span>
                </div>
                <StagedSectionActions
                    count=staged_count
                    bulk_busy=bulk_busy
                    set_bulk_busy=set_bulk_busy
                    entries_for_action=staged_list_for_action
                />
            </div>

            {move || if expanded.get() {
                view! {
                    <For
                        each=move || staged_list.get_value()
                        key=|e| {
                            format!(
                                "{}:{}:{:?}:{}",
                                e.doc_id
                                    .map(|doc_id| doc_id.to_string())
                                    .unwrap_or_default(),
                                e.path,
                                e.status,
                                e.renamed_from.clone().unwrap_or_default()
                            )
                        }
                        children=move |e| view! { <ChangeItem entry=e is_staged=true /> }
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }.into_any()
}
