//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! External Changes sidebar view.

use crate::components::icons::{AlertTriangle, Check, FileText, Minus, Plus, RotateCcw};
use crate::components::sidebar::source_control::changes::should_request_changes;
use crate::hooks::use_core::ExternalChangesContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub fn ExternalChangesView() -> impl IntoView {
    let core = expect_context::<ExternalChangesContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let read_block = core.read_block;

    Effect::new({
        let core = core.clone();
        move |_| {
            if !should_request_changes(
                core.current_repo_id.get().is_some(),
                core.active_branch.get().is_some(),
                core.pending_branch_switch.get().is_some(),
                core.pending_repo_switch.get().is_some(),
                read_block.get().is_some(),
            ) {
                return;
            }
            core.on_get_changes.run(());
        }
    });

    let core_for_apply_title = core.clone();
    let core_for_apply_disabled = core.clone();
    let core_for_apply_click = core.clone();

    view! {
        <div
            class="h-full w-full bg-sidebar flex flex-col font-sans select-none overflow-hidden text-[13px] text-primary"
            data-deve-external-changes-view="true"
        >
            <div class="flex min-h-10 items-center justify-between gap-2 border-b border-default px-3">
                <div class="min-w-0 truncate text-[11px] font-bold uppercase tracking-normal">
                    {move || t::external_changes::title(locale.get())}
                </div>
                <button
                    type="button"
                    class="inline-flex h-11 shrink-0 items-center gap-1 rounded border border-border px-3 text-[11px] font-medium text-primary hover:bg-hover disabled:cursor-not-allowed disabled:opacity-50 md:h-7 md:px-2"
                    data-deve-external-apply="true"
                    title=move || apply_title(locale.get(), &core_for_apply_title)
                    disabled=move || !can_apply_to_ledger(&core_for_apply_disabled)
                    on:click=move |_| core_for_apply_click.on_apply_to_ledger.run(())
                >
                    <Check class="h-3.5 w-3.5" />
                    <span class="max-w-28 truncate">{move || t::external_changes::apply_to_ledger(locale.get())}</span>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto">
                {move || {
                    let staged = core.staged_changes.get();
                    let unstaged = core.unstaged_changes.get();
                    if staged.is_empty() && unstaged.is_empty() {
                        view! {
                            <div class="px-3 py-6 text-center text-xs text-muted">
                                {t::external_changes::no_changes(locale.get())}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <ExternalChangesSection
                                title=t::external_changes::staged(locale.get()).to_string()
                                entries=staged
                                is_staged=true
                                core=core.clone()
                                locale
                            />
                            <ExternalChangesSection
                                title=t::external_changes::pending(locale.get()).to_string()
                                entries=unstaged
                                is_staged=false
                                core=core.clone()
                                locale
                            />
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn ExternalChangesSection(
    title: String,
    entries: Vec<ChangeEntry>,
    is_staged: bool,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let count = entries.len();
    let entries = StoredValue::new(entries);

    if count == 0 {
        return view! {}.into_any();
    }

    let section_marker = title.clone();
    let section_title = title;

    view! {
        <section data-deve-external-section=section_marker>
            <div
                class="flex min-h-11 items-center justify-between px-3 text-[11px] font-bold uppercase text-muted md:min-h-7"
                data-deve-mobile-touch-target="external-changes-section-header"
            >
                <span class="truncate">{section_title}</span>
                <span>{count}</span>
            </div>
            <For
                each=move || entries.get_value()
                key=external_change_key
                children=move |entry| {
                    external_change_row(entry, is_staged, core.clone(), locale)
                }
            />
        </section>
    }.into_any()
}

fn external_change_row(
    entry: ChangeEntry,
    is_staged: bool,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> AnyView {
    let entry_store = StoredValue::new(entry);
    let display_path = entry_store.get_value().path;
    let display_name = file_name(&display_path);
    let directory = directory_name(&display_path);

    view! {
        <div
            class="group flex min-h-11 items-center gap-2 px-3 py-1 hover:bg-hover md:min-h-9"
            data-deve-external-changes-row=display_path.clone()
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
                {move || {
                    let overlaps = external_change_is_overlap_blocked(&entry_store.get_value());
                    if overlaps {
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
                        }.into_any()
                    } else if is_staged {
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
                        }.into_any()
                    } else {
                        let core_for_discard_disabled = core.clone();
                        let core_for_discard_click = core.clone();
                        let core_for_stage_disabled = core.clone();
                        let core_for_stage_click = core.clone();
                        view! {
                            <ExternalChangeIconButton
                                title=Signal::derive(move || {
                                    t::external_changes::discard(locale.get()).to_string()
                                })
                                disabled=Signal::derive(move || !core_for_discard_disabled.can_write.get())
                                on_click=Callback::new(move |_| {
                                    core_for_discard_click.on_discard_file.run(entry_store.get_value())
                                })
                            >
                                <RotateCcw class="h-3.5 w-3.5" />
                            </ExternalChangeIconButton>
                            <ExternalChangeIconButton
                                title=Signal::derive(move || {
                                    t::external_changes::stage(locale.get()).to_string()
                                })
                                disabled=Signal::derive(move || !core_for_stage_disabled.can_write.get())
                                on_click=Callback::new(move |_| {
                                    core_for_stage_click.on_stage_file.run(entry_store.get_value())
                                })
                            >
                                <Plus class="h-3.5 w-3.5" />
                            </ExternalChangeIconButton>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }.into_any()
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
            on:click=move |_| on_click.run(())
        >
            {children()}
        </button>
    }
}

fn can_apply_to_ledger(core: &ExternalChangesContext) -> bool {
    let staged = core.staged_changes.get();
    core.can_write.get() && !staged.is_empty() && !staged.iter().any(|entry| entry.has_conflict)
}

fn apply_title(locale: Locale, core: &ExternalChangesContext) -> String {
    if can_apply_to_ledger(core) {
        return t::external_changes::apply_to_ledger(locale).to_string();
    }
    t::external_changes::apply_to_ledger_disabled(locale).to_string()
}

fn external_change_is_overlap_blocked(entry: &ChangeEntry) -> bool {
    entry.has_conflict
}

fn external_change_key(entry: &ChangeEntry) -> String {
    format!(
        "{}:{}:{}:{:?}:{:?}",
        entry
            .doc_id
            .map(|doc_id| doc_id.to_string())
            .unwrap_or_default(),
        entry.path,
        entry.renamed_from.clone().unwrap_or_default(),
        entry.status,
        entry.domain,
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
mod tests {
    use super::{directory_name, external_change_is_overlap_blocked, file_name};
    use deve_core::source_control::{ChangeDomain, ChangeEntry, ChangeStatus};

    fn entry(path: &str, has_conflict: bool, domain: ChangeDomain) -> ChangeEntry {
        ChangeEntry {
            path: path.to_string(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Modified,
            has_conflict,
            domain,
            base_seq: None,
            target_seq: None,
        }
    }

    #[test]
    fn overlap_state_comes_from_backend_conflict_flag() {
        let blocked = entry("notes/a.md", true, ChangeDomain::WorkingDirectory);
        let clean = entry("notes/a.md", false, ChangeDomain::WorkingDirectory);

        assert!(external_change_is_overlap_blocked(&blocked));
        assert!(!external_change_is_overlap_blocked(&clean));
    }

    #[test]
    fn path_display_splits_name_and_directory() {
        assert_eq!(file_name("notes/a.md"), "a.md");
        assert_eq!(directory_name("notes/a.md"), "notes");
        assert_eq!(file_name("a.md"), "a.md");
        assert_eq!(directory_name("a.md"), "");
    }

    #[test]
    fn mobile_external_changes_touch_targets_min_size_bound() {
        let source = include_str!("external_changes.rs");

        assert!(source.contains("data-deve-mobile-touch-target=\"external-changes-action\""));
        assert!(
            source.contains("data-deve-mobile-touch-target=\"external-changes-section-header\"")
        );
        assert!(source.contains("class=\"inline-flex h-11 w-11"));
        assert!(source.contains("md:h-7 md:w-7"));
        assert!(source.contains("class=\"group flex min-h-11"));
        assert!(source.contains("md:min-h-9"));
    }

    #[test]
    fn external_changes_keeps_its_action_surface_separate() {
        let source = include_str!("external_changes.rs");

        assert!(!source.contains(concat!("change_item_", "action_surface")));
        assert!(!source.contains(concat!("ChangeItem", "ActionSurface")));
        assert!(!source.contains(concat!("change_item_", "conflict_actions")));
    }
}
