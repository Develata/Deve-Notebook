//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
//! Confirmed ledger changes are already authoritative ledger facts, not
//! pending_fs_ops. First batch exposes diff + whole-anchor commit only.

use super::change_item::ChangeItem;
use crate::components::icons::ChevronRight;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub fn ConfirmedSection(confirmed: Vec<ChangeEntry>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let expanded = RwSignal::new(true);
    let confirmed_count = confirmed.len();
    let confirmed_list = StoredValue::new(confirmed);

    if confirmed_count == 0 {
        return view! {}.into_any();
    }

    view! {
        <div>
            <div
                class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-hover"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <div class="flex items-center">
                    <span class=move || format!("w-4 h-4 flex items-center justify-center text-primary transition-transform {}", if expanded.get() { "rotate-90" } else { "" })>
                        <ChevronRight class="w-3 h-3" />
                    </span>
                    <span class="text-[11px] font-bold text-primary uppercase">
                        {move || t::source_control::confirmed_ledger_changes(locale.get())}
                    </span>
                </div>
                <span class="text-[11px] text-muted pr-2">{confirmed_count}</span>
            </div>
            <div
                class="px-8 pb-1 text-[11px] leading-4 text-muted"
                data-deve-sc-confirmed-ledger-hint="true"
            >
                {move || t::source_control::confirmed_ledger_hint(locale.get())}
            </div>

            {move || if expanded.get() {
                view! {
                    <For
                        each=move || confirmed_list.get_value()
                        key=|e| {
                            format!(
                                "{:?}:{}:{}:{:?}:{}:{}:{}",
                                e.domain,
                                e.doc_id
                                    .map(|doc_id| doc_id.to_string())
                                    .unwrap_or_default(),
                                e.path,
                                e.status,
                                e.renamed_from.clone().unwrap_or_default(),
                                e.base_seq.unwrap_or_default(),
                                e.target_seq.unwrap_or_default()
                            )
                        }
                        children=move |e| view! { <ChangeItem entry=e is_staged=false /> }
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }.into_any()
}
