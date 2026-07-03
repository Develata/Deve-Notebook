//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! External Changes row rendering and action surface.

use crate::components::icons::{AlertTriangle, FileText, Minus, Plus, RotateCcw};
use crate::hooks::use_core::ExternalChangesContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

pub(super) fn external_change_row(
    entry: ChangeEntry,
    is_staged: bool,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> AnyView {
    let entry_store = StoredValue::new(entry);
    let display_path = entry_store.get_value().path;
    let display_name = file_name(&display_path);
    let directory = directory_name(&display_path);
    let core_for_row_click = core.clone();

    view! {
        <div
            class="group flex min-h-11 cursor-pointer items-center gap-2 px-3 py-1 hover:bg-hover md:min-h-9"
            data-deve-external-changes-row=display_path.clone()
            on:click=move |_| core_for_row_click.on_get_doc_diff.run(entry_store.get_value())
        >
            <FileText class="h-3.5 w-3.5 shrink-0 text-muted" />
            <div class="min-w-0 flex-1">
                <div class="flex min-w-0 items-center gap-1.5">
                    <span class="truncate text-[12px] text-primary">{display_name}</span>
                    <span class="shrink truncate text-[11px] text-muted">{directory}</span>
                </div>
                <Show when=move || external_change_is_overlap_blocked(&entry_store.get_value())>
                    <div
                        class="mt-0.5 flex items-center gap-1 text-[11px] leading-4 text-warning"
                        data-deve-external-overlap="true"
                    >
                        <AlertTriangle class="h-3 w-3" />
                        <span>{move || t::external_changes::overlap_blocked(locale.get())}</span>
                    </div>
                </Show>
            </div>
            <div class="flex shrink-0 items-center gap-1 opacity-100 md:opacity-0 md:group-hover:opacity-100 md:group-focus-within:opacity-100">
                <ExternalChangeIconButton
                    title=Signal::derive(move || {
                        t::external_changes::open_diff(locale.get()).to_string()
                    })
                    disabled=Signal::derive(|| false)
                    on_click=Callback::new({
                        let core = core.clone();
                        move |_| core.on_get_doc_diff.run(entry_store.get_value())
                    })
                >
                    <FileText class="h-3.5 w-3.5" />
                </ExternalChangeIconButton>
                {move || row_write_actions(entry_store, is_staged, core.clone(), locale)}
            </div>
        </div>
    }.into_any()
}

fn row_write_actions(
    entry_store: StoredValue<ChangeEntry>,
    is_staged: bool,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> AnyView {
    if external_change_is_overlap_blocked(&entry_store.get_value()) {
        return discard_action(entry_store, core, locale).into_any();
    }
    if is_staged {
        return unstage_action(entry_store, core, locale).into_any();
    }
    view! {
        {discard_action(entry_store, core.clone(), locale)}
        {stage_action(entry_store, core, locale)}
    }
    .into_any()
}

fn discard_action(
    entry_store: StoredValue<ChangeEntry>,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let core_for_disabled = core.clone();
    let core_for_click = core.clone();

    view! {
        <ExternalChangeIconButton
            title=Signal::derive(move || {
                t::external_changes::discard(locale.get()).to_string()
            })
            disabled=Signal::derive(move || !core_for_disabled.can_write.get())
            on_click=Callback::new(move |_| {
                core_for_click.on_discard_file.run(entry_store.get_value())
            })
        >
            <RotateCcw class="h-3.5 w-3.5" />
        </ExternalChangeIconButton>
    }
}

fn stage_action(
    entry_store: StoredValue<ChangeEntry>,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let core_for_disabled = core.clone();
    let core_for_click = core.clone();

    view! {
        <ExternalChangeIconButton
            title=Signal::derive(move || {
                t::external_changes::stage(locale.get()).to_string()
            })
            disabled=Signal::derive(move || !core_for_disabled.can_write.get())
            on_click=Callback::new(move |_| {
                core_for_click.on_stage_file.run(entry_store.get_value())
            })
        >
            <Plus class="h-3.5 w-3.5" />
        </ExternalChangeIconButton>
    }
}

fn unstage_action(
    entry_store: StoredValue<ChangeEntry>,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let core_for_disabled = core.clone();
    let core_for_click = core.clone();

    view! {
        <ExternalChangeIconButton
            title=Signal::derive(move || {
                t::external_changes::unstage(locale.get()).to_string()
            })
            disabled=Signal::derive(move || !core_for_disabled.can_write.get())
            on_click=Callback::new(move |_| {
                core_for_click.on_unstage_file.run(entry_store.get_value())
            })
        >
            <Minus class="h-3.5 w-3.5" />
        </ExternalChangeIconButton>
    }
}

#[component]
fn ExternalChangeIconButton(
    #[prop(into)] title: Signal<String>,
    #[prop(into)] disabled: Signal<bool>,
    on_click: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="inline-flex h-11 w-11 items-center justify-center rounded text-muted hover:bg-hover hover:text-primary disabled:cursor-not-allowed disabled:opacity-40 md:h-7 md:w-7"
            title=move || title.get()
            aria-label=move || title.get()
            disabled=move || disabled.get()
            data-deve-mobile-touch-target="external-changes-action"
            on:click=move |event| {
                event.stop_propagation();
                on_click.run(());
            }
        >
            {children()}
        </button>
    }
}

fn external_change_is_overlap_blocked(entry: &ChangeEntry) -> bool {
    entry.has_conflict
}

pub(super) fn external_change_key(entry: &ChangeEntry) -> String {
    format!(
        "{}:{}:{}:{:?}:{:?}:{}:{:?}:{:?}",
        entry
            .doc_id
            .map(|doc_id| doc_id.to_string())
            .unwrap_or_default(),
        entry.path,
        entry.renamed_from.clone().unwrap_or_default(),
        entry.status,
        entry.domain,
        entry.has_conflict,
        entry.base_seq,
        entry.target_seq,
    )
}

fn file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn directory_name(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "row_tests.rs"]
mod tests;
