// apps\web\src\components\sidebar\source_control
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # UnstagedSection 组件 (工作区组件)
//!
//! 渲染工作区 (Unstaged Changes) 的文件列表。

use super::change_item::ChangeItem;
use super::touch_target::section_header_class;
use super::unstaged_section_actions::UnstagedSectionActions;
use crate::components::icons::ChevronRight;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

/// 工作区组件
#[component]
pub fn UnstagedSection(unstaged: Vec<ChangeEntry>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (bulk_busy, set_bulk_busy) = signal(false);
    let expanded = RwSignal::new(true);

    let unstaged_count = unstaged.len();
    let unstaged_list = StoredValue::new(unstaged.clone());
    let unstaged_list_for_stage = StoredValue::new(unstaged.clone());

    Effect::new(move |_| {
        let _ = core.unstaged_changes.get();
        let _ = core.staged_changes.get();
        let _ = core.confirmed_changes.get();
        set_bulk_busy.set(false);
    });

    Effect::new(move |_| {
        let notice = core.notice.get();
        let write_block = core.write_block.get();
        if notice.is_some() || write_block.is_some() {
            set_bulk_busy.set(false);
        }
    });

    if unstaged_count == 0 {
        return view! {}.into_any();
    }

    view! {
        <div>
            <div
                class=section_header_class()
                data-deve-mobile-touch-target="source-control-section-header"
            >
                <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center rounded-sm text-left focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/40"
                    data-deve-sc-section-toggle="unstaged"
                    aria-expanded=move || expanded.get().to_string()
                    aria-controls="source-control-unstaged-changes-panel"
                    on:click=move |_| expanded.update(|v| *v = !*v)
                >
                    <span class=move || format!("w-4 h-4 flex items-center justify-center text-primary transition-transform {}", if expanded.get() { "rotate-90" } else { "" })>
                        <ChevronRight class="w-3 h-3" />
                    </span>
                    <span class="truncate text-[11px] font-bold text-primary uppercase">
                        {move || t::source_control::changes(locale.get())}
                    </span>
                </button>
                <UnstagedSectionActions
                    count=unstaged_count
                    bulk_busy=bulk_busy
                    set_bulk_busy=set_bulk_busy
                    entries_for_stage=unstaged_list_for_stage
                />
            </div>

            <div
                id="source-control-unstaged-changes-panel"
                data-deve-sc-section-body="unstaged"
                hidden=move || !expanded.get()
            >
                <For
                    each=move || unstaged_list.get_value()
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
                    children=move |e| view! { <ChangeItem entry=e is_staged=false /> }
                />
            </div>
        </div>
    }.into_any()
}
