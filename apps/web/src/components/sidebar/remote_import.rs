//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!   - 12_source_control_ui#remote-import-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! Remote Import sibling view. It renders only typed backend projections and
//! dispatches whole-session intents through `remote_import_client`.

use crate::components::diff_view::ReadonlyDiffView;
use crate::components::icons::{Check, Download, RefreshCw, Trash2};
use crate::i18n::{Locale, t};
use crate::runtime::remote_import_client::{
    RemoteImportAvailability, RemoteImportClient, RemoteImportProjection,
};
use deve_core::protocol::{RemoteImportSessionView, RemoteImportState, RemoteProjectionProvider};
use leptos::prelude::*;

#[component]
pub fn RemoteImportView() -> impl IntoView {
    let client = expect_context::<RemoteImportClient>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let projection = client.projection();

    let list_client = client.clone();
    Effect::new(move |_| {
        let availability = list_client.synchronize_scope();
        if matches!(availability, RemoteImportAvailability::Ready { .. }) {
            let _ = list_client.list();
        }
    });

    let webdav = client.clone();
    let s3 = client.clone();
    let selected_client = client.clone();

    view! {
        <div
            class="h-full w-full bg-sidebar flex flex-col font-sans select-none overflow-hidden text-[13px] text-primary"
            data-deve-remote-import-view="true"
        >
            <div class="flex min-h-10 items-center justify-between gap-2 border-b border-default px-3">
                <div class="min-w-0 truncate text-[11px] font-bold uppercase tracking-normal">
                    {move || t::remote_import::title(locale.get())}
                </div>
                <div class="flex items-center gap-1">
                    <button
                        type="button"
                        class="inline-flex min-h-[44px] items-center gap-1 rounded border border-border px-2 text-[11px] hover:bg-hover md:min-h-[28px]"
                        data-deve-remote-import-prepare="webdav"
                        aria-label=move || t::remote_import::prepare_webdav(locale.get())
                        title=move || t::remote_import::prepare_webdav(locale.get())
                        disabled=move || !matches!(
                            projection.get().availability,
                            RemoteImportAvailability::Ready { .. }
                        )
                        on:click=move |_| { let _ = webdav.prepare(RemoteProjectionProvider::WebDav); }
                    >
                        <Download class="h-3.5 w-3.5" />
                        <span class="md:hidden">
                            {move || t::remote_import::prepare_webdav(locale.get())}
                        </span>
                    </button>
                    <button
                        type="button"
                        class="inline-flex min-h-[44px] items-center gap-1 rounded border border-border px-2 text-[11px] hover:bg-hover md:min-h-[28px]"
                        data-deve-remote-import-prepare="s3"
                        aria-label=move || t::remote_import::prepare_s3(locale.get())
                        title=move || t::remote_import::prepare_s3(locale.get())
                        disabled=move || !matches!(
                            projection.get().availability,
                            RemoteImportAvailability::Ready { .. }
                        )
                        on:click=move |_| { let _ = s3.prepare(RemoteProjectionProvider::S3); }
                    >
                        <Download class="h-3.5 w-3.5" />
                        <span class="md:hidden">
                            {move || t::remote_import::prepare_s3(locale.get())}
                        </span>
                    </button>
                </div>
            </div>

            <div class="flex-1 min-h-0 overflow-y-auto">
                {move || remote_import_body(selected_client.clone(), locale, projection.get())}
            </div>
        </div>
    }
}

fn remote_import_body(
    client: RemoteImportClient,
    locale: RwSignal<Locale>,
    projection: RemoteImportProjection,
) -> AnyView {
    match projection.availability {
        RemoteImportAvailability::Offline => {
            scope_notice(t::remote_import::offline(locale.get()), "offline")
        }
        RemoteImportAvailability::NoRepo => {
            scope_notice(t::remote_import::no_repo(locale.get()), "no-repo")
        }
        RemoteImportAvailability::ScopeTransitioning => scope_notice(
            t::remote_import::scope_transitioning(locale.get()),
            "scope-transitioning",
        ),
        RemoteImportAvailability::Ready {
            workspace_ingestion_blocked,
        } => {
            let error = projection.error;
            let sessions = if projection.sessions.is_empty() {
                let copy = if projection.pending.list {
                    t::remote_import::request_pending(locale.get())
                } else {
                    t::remote_import::no_sessions(locale.get())
                };
                view! {
                    <div class="px-3 py-6 text-center text-xs text-muted">{copy}</div>
                }
                .into_any()
            } else {
                let rows = projection
                    .sessions
                    .iter()
                    .cloned()
                    .map(|session| session_row(client.clone(), locale, session))
                    .collect::<Vec<_>>();
                view! {
                    <div class="border-b border-default py-1" data-deve-remote-import-sessions="true">
                        {rows}
                    </div>
                }
                .into_any()
            };
            view! {
                <div>
                    {workspace_ingestion_blocked.then(|| view! {
                        <div
                            class="border-b border-default bg-warning/10 px-3 py-2 text-[11px] text-warning"
                            data-deve-remote-import-ingestion="unavailable"
                        >
                            {t::remote_import::workspace_ingestion_unavailable(locale.get())}
                        </div>
                    })}
                    {error.map(|code| view! {
                        <div class="border-b border-default px-3 py-2 text-[11px] text-danger" data-deve-remote-import-error="typed">
                            {t::server_error::message(locale.get(), code)}
                        </div>
                    })}
                    {sessions}
                    {selected_session_panel(client, locale, projection)}
                </div>
            }
            .into_any()
        }
    }
}

fn scope_notice(copy: &'static str, marker: &'static str) -> AnyView {
    view! {
        <div
            class="px-3 py-6 text-center text-xs text-muted"
            data-deve-remote-import-availability=marker
        >
            {copy}
        </div>
    }
    .into_any()
}

fn session_row(
    client: RemoteImportClient,
    locale: RwSignal<Locale>,
    session: RemoteImportSessionView,
) -> impl IntoView {
    let session_id = session.session_id;
    let revision = session.revision;
    let entry_count = session.entry_count;
    let state = session.state;
    let revision_label = revision.map(|revision| format!("r{}", revision.get()));
    view! {
        <button
            type="button"
            class="flex min-h-[44px] w-full items-center justify-between gap-2 px-3 py-2 text-left hover:bg-hover"
            data-deve-remote-import-session=session_id.get().to_string()
            on:click=move |_| {
                let _ = client.show(session_id, revision);
                if let Some(revision) = revision {
                    let _ = client.first_page(session_id, revision);
                }
            }
        >
            <span class="min-w-0 truncate font-mono text-[11px]">
                {session_id.get().to_string()}
            </span>
            <span class="shrink-0 text-[11px] text-muted">
                {move || format!(
                    "{}{} · {entry_count}",
                    t::remote_import::state(locale.get(), state),
                    revision_label
                        .as_deref()
                        .map(|revision| format!(" · {revision}"))
                        .unwrap_or_default()
                )}
            </span>
        </button>
    }
}

fn selected_session_panel(
    client: RemoteImportClient,
    locale: RwSignal<Locale>,
    projection: RemoteImportProjection,
) -> AnyView {
    let Some(session) = projection.selected_session.clone() else {
        return view! {
            <div class="px-3 py-5 text-center text-xs text-muted">
                {if projection.pending.show {
                    t::remote_import::request_pending(locale.get())
                } else {
                    t::remote_import::select_session(locale.get())
                }}
            </div>
        }
        .into_any();
    };
    let session_id = session.session_id;
    let revision = session.revision;
    let workspace_ingestion_blocked = matches!(
        projection.availability,
        RemoteImportAvailability::Ready {
            workspace_ingestion_blocked: true
        }
    );
    let can_apply = session.state == RemoteImportState::Ready
        && session.blockers.is_empty()
        && revision.is_some()
        && !workspace_ingestion_blocked
        && !projection.pending.apply
        && !projection.selected_apply_completed();
    let apply_outcome = projection.apply_outcome_for(session_id, revision);
    let refresh_client = client.clone();
    let apply_client = client.clone();
    let discard_client = client.clone();
    let more_client = client.clone();

    view! {
        <div data-deve-remote-import-selected=session_id.get().to_string()>
            <div class="flex flex-wrap items-center gap-1 border-b border-default px-3 py-2">
                <button
                    type="button"
                    class="inline-flex min-h-[44px] items-center gap-1 rounded border border-border px-2 text-[11px] hover:bg-hover disabled:opacity-50 md:min-h-[28px]"
                    data-deve-remote-import-refresh="true"
                    disabled=revision.is_none() || projection.pending.refresh
                    on:click=move |_| {
                        if let Some(revision) = revision {
                            let _ = refresh_client.refresh(session_id, revision);
                        }
                    }
                >
                    <RefreshCw class="h-3.5 w-3.5" />
                    {t::remote_import::refresh(locale.get())}
                </button>
                <button
                    type="button"
                    class="inline-flex min-h-[44px] items-center gap-1 rounded border border-border px-2 text-[11px] hover:bg-hover disabled:opacity-50 md:min-h-[28px]"
                    data-deve-remote-import-apply="true"
                    disabled=!can_apply
                    on:click=move |_| {
                        if let Some(revision) = revision {
                            let _ = apply_client.apply(session_id, revision);
                        }
                    }
                >
                    <Check class="h-3.5 w-3.5" />
                    {t::remote_import::apply(locale.get())}
                </button>
                <button
                    type="button"
                    class="inline-flex min-h-[44px] items-center gap-1 rounded border border-border px-2 text-[11px] hover:bg-hover md:min-h-[28px]"
                    data-deve-remote-import-discard="true"
                    disabled=projection.pending.discard
                    on:click=move |_| { let _ = discard_client.discard(session_id, revision); }
                >
                    <Trash2 class="h-3.5 w-3.5" />
                    {t::remote_import::discard(locale.get())}
                </button>
            </div>

            {projection.pending.any().then(|| view! {
                <div
                    class="border-b border-default px-3 py-2 text-[11px] text-muted"
                    data-deve-remote-import-pending="true"
                >
                    {t::remote_import::request_pending(locale.get())}
                </div>
            })}

            {(!session.blockers.is_empty()).then(|| view! {
                <div class="border-b border-default bg-warning/10 px-3 py-2 text-[11px]" data-deve-remote-import-blockers="backend-typed">
                    <div class="font-medium">{t::remote_import::blocked(locale.get())}</div>
                    <ul class="mt-1 list-disc pl-4 text-muted">
                        {session.blockers.into_iter().map(|blocker| view! {
                            <li>{t::remote_import::blocker(locale.get(), blocker)}</li>
                        }).collect::<Vec<_>>()}
                    </ul>
                </div>
            })}

            {session.cleanup_pending.then(|| view! {
                <div class="border-b border-default px-3 py-2 text-[11px] text-warning">
                    {t::remote_import::cleanup_pending(locale.get())}
                </div>
            })}

            {apply_outcome.map(|projection_outcome| view! {
                <div class="border-b border-default px-3 py-2 text-[11px]" data-deve-remote-import-apply-outcome="backend-typed">
                    {t::remote_import::projection_outcome(locale.get(), projection_outcome)}
                </div>
            })}

            {if projection.entries.is_empty() {
                view! {
                    <div class="px-3 py-5 text-center text-xs text-muted">
                        {t::remote_import::no_entries(locale.get())}
                    </div>
                }.into_any()
            } else {
                view! {
                    <div data-deve-remote-import-entries="backend-typed">
                        {projection.entries.into_iter().map(|entry| {
                            let entry_id = entry.entry_id.clone();
                            let label = entry.display_label.clone();
                            let blockers = entry.blockers;
                            let diff_client = client.clone();
                            view! {
                                <button
                                    type="button"
                                    class="flex min-h-[44px] w-full items-center justify-between gap-2 border-b border-default px-3 py-2 text-left hover:bg-hover"
                                    data-deve-remote-import-entry=entry_id.as_str().to_string()
                                    on:click=move |_| {
                                        if let Some(revision) = revision {
                                            let _ = diff_client.diff(session_id, revision, entry_id.clone());
                                        }
                                    }
                                >
                                    <span class="min-w-0">
                                        <span class="block truncate">{label}</span>
                                        {(!blockers.is_empty()).then(|| view! {
                                            <span class="block truncate text-[11px] text-warning">
                                                {blockers.into_iter().map(|blocker| {
                                                    t::remote_import::blocker(locale.get(), blocker)
                                                }).collect::<Vec<_>>().join(" · ")}
                                            </span>
                                        })}
                                    </span>
                                    <span class="shrink-0 text-[11px] text-muted">
                                        {t::remote_import::change_kind(locale.get(), entry.change_kind)}
                                    </span>
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            {projection.next_cursor.is_some().then(|| view! {
                <button
                    type="button"
                    class="min-h-[44px] w-full border-b border-default px-3 py-2 text-xs text-accent hover:bg-hover"
                    data-deve-remote-import-next-page="true"
                    disabled=projection.pending.page
                    on:click=move |_| { let _ = more_client.next_page(); }
                >
                    {t::remote_import::load_more(locale.get())}
                </button>
            })}

            {projection.diff.map(|diff| {
                let close_client = client.clone();
                let diff_entry_id = diff.entry_id.as_str().to_string();
                let diff_session_id = diff.session_id.get().to_string();
                let diff_revision = diff.revision.get().to_string();
                let diff_kind = diff.change_kind;
                let diff_blockers = diff.blockers;
                let display_path = diff.display_label;
                let diff_projection = diff.projection;
                view! {
                    <div
                        class="border-t border-default"
                        data-deve-remote-import-diff="backend-typed"
                        data-deve-remote-import-diff-entry=diff_entry_id
                        data-deve-remote-import-diff-session=diff_session_id
                        data-deve-remote-import-diff-revision=diff_revision
                    >
                        <div class="border-b border-default px-3 py-1 text-[11px] text-muted">
                            {t::remote_import::change_kind(locale.get(), diff_kind)}
                            {diff_blockers.into_iter().map(|blocker| view! {
                                <span class="ml-2 text-warning">{t::remote_import::blocker(locale.get(), blocker)}</span>
                            }).collect::<Vec<_>>()}
                        </div>
                        <div class="h-80">
                            <ReadonlyDiffView
                                display_path
                                projection=diff_projection
                                force_unified=true
                                mobile=true
                                on_close=Callback::new(move |_| close_client.clear_diff())
                            />
                        </div>
                    </div>
                }
            })}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    #[test]
    fn remote_import_mobile_actions_use_real_44px_minimums() {
        let source = include_str!("remote_import.rs");
        let invalid_spacing_token = ["min-h", "-11"].concat();
        let mobile_minimum = ["min-h-", "[44px]"].concat();
        let desktop_minimum = ["md:min-h-", "[28px]"].concat();
        let desktop_hidden_label = ["class=\"md:", "hidden\""].concat();

        assert!(!source.contains(&invalid_spacing_token));
        assert_eq!(source.matches(&mobile_minimum).count(), 8);
        assert_eq!(source.matches(&desktop_minimum).count(), 5);
        assert_eq!(source.matches(&desktop_hidden_label).count(), 2);
    }
}
