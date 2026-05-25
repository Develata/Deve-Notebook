//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
use crate::components::icons::{AlertTriangle, FileText};
use crate::components::sidebar::source_control::change_item_counterpart::{
    counterpart_badge_text, counterpart_badge_title, find_counterpart_kind,
};
use crate::components::sidebar::source_control::change_item_meta::ChangeItemMeta;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub fn ChangeItemContent(
    locale: RwSignal<Locale>,
    entry: ChangeEntry,
    is_staged: bool,
    meta: ChangeItemMeta,
    has_conflict: bool,
    staged_changes: ReadSignal<Vec<ChangeEntry>>,
    unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
) -> impl IntoView {
    let entry_for_counterpart = entry;

    view! {
        <div class="flex items-center gap-1.5 flex-1 overflow-hidden">
            <FileText class=format!("w-3.5 h-3.5 min-w-3.5 {}", meta.file_icon_class) />

            <span class="truncate">{meta.display_name}</span>
            <span class="text-xs text-muted truncate shrink-0 ml-1">{meta.directory}</span>
            {move || {
                let counterpart = find_counterpart_kind(
                    &entry_for_counterpart,
                    is_staged,
                    &staged_changes.get(),
                    &unstaged_changes.get(),
                );
                counterpart.map(|kind| {
                    let locale_value = locale.get();
                    view! {
                        <span
                            class="ml-1 shrink-0 rounded border border-border px-1 py-px text-[10px] font-semibold text-muted"
                            title=counterpart_badge_title(kind, locale_value)
                        >
                            {counterpart_badge_text(kind, locale_value)}
                        </span>
                    }
                })
            }}
            {if has_conflict {
                view! {
                    <span
                        class="ml-auto shrink-0"
                        title=move || t::source_control::git_import_conflict_title(locale.get())
                    >
                        <AlertTriangle class="w-3 h-3 text-warning" />
                    </span>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
        <span class=format!("{} text-[11px] font-bold w-3 text-center shrink-0", meta.color_class)>
            {meta.icon_char}
        </span>
    }
}
