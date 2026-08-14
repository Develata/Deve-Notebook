// apps/web/src/components/mobile_layout/content.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 10_rendering#large-document-runtime
//!
//! # Mobile Content

use crate::components::dashboard::Dashboard;
use crate::editor::Editor;
use crate::i18n::{Locale, t};
use crate::runtime::source_control_client::diff_session::DiffProjectionIntent;
use crate::runtime::{
    rendering_client::RenderingClient, scope_client::ScopeClient, session_client::SessionClient,
    source_control_client::SourceControlClient,
};
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

pub(super) fn mobile_content_keyboard_overlay_style(offset: i32) -> String {
    format!("padding-bottom: {}px;", offset.max(0))
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MobileContentSurface {
    Editor,
    Dashboard,
}

#[cfg(test)]
pub(crate) fn mobile_content_surface_after_diff_close(
    has_current_doc: bool,
    pending_branch_switch: bool,
    pending_repo_switch: bool,
) -> MobileContentSurface {
    if has_current_doc && !pending_branch_switch && !pending_repo_switch {
        MobileContentSurface::Editor
    } else {
        MobileContentSurface::Dashboard
    }
}

#[component]
pub fn MobileContent(
    drawer_open: Signal<bool>,
    current_editor_doc: Signal<Option<DocId>>,
    keyboard_overlay_offset: Signal<i32>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let rendering = expect_context::<RenderingClient>();
    let source_control = expect_context::<SourceControlClient>();
    let scope = expect_context::<ScopeClient>();
    let session = expect_context::<SessionClient>();
    let on_stats = rendering.on_stats;
    let diff_content = source_control.diff_content;
    let set_diff_content = source_control.set_diff_content;
    let current_scope_nonce = scope.current_scope_nonce;
    let is_spectator = scope.is_spectator;
    let sync_banner = session.sync_banner;
    let ws = session.ws.clone();
    let compute_ws = ws.clone();
    let on_compute_projection = Callback::new(move |intent: DiffProjectionIntent| {
        set_diff_content.update(|current| {
            if let Some(current) = current {
                current.begin_compute(&intent);
            }
        });
        compute_ws.send(ClientMessage::ComputeDiffProjection {
            request_id: intent.request_id,
            revision: intent.revision,
            base_content: intent.base_content,
            target_content: intent.target_content,
            scope_nonce: Some(current_scope_nonce.get_untracked()),
        });
    });
    let on_persist_draft = Callback::new(move |draft: String| {
        set_diff_content.update(|current| {
            if let Some(current) = current {
                current.persist_draft(draft);
            }
        });
    });
    view! {
        <div
            data-deve-native-keyboard-overlay=move || keyboard_overlay_offset.get().to_string()
            data-deve-mobile-work-edit-swipe-surface=move || {
                (current_editor_doc.get().is_some() && diff_content.get().is_none()).to_string()
            }
            class="relative flex-1 overflow-hidden transition-opacity flex flex-col"
            style=move || mobile_content_keyboard_overlay_style(keyboard_overlay_offset.get())
            class:pointer-events-none=move || drawer_open.get()
            class:opacity-80=move || drawer_open.get()
        >
            <Show when=move || is_spectator.get() && sync_banner.get().is_none()>
                <div class="h-6 px-3 flex items-center text-[11px] font-medium text-orange-900 bg-orange-200 border-b border-orange-300">
                    {move || t::common::read_only_mode(locale.get())}
                </div>
            </Show>
            <div class="flex-1 min-h-0 overflow-hidden">
                {move || {
                    if let Some(session) = diff_content.get() {
                            let merge_conflict = session.merge_conflict.clone();
                            let resolve_ws = ws.clone();
                            let on_resolve = merge_conflict.clone().map(|conflict| {
                                let resolve_ws = resolve_ws.clone();
                                Callback::new(move |(action, result_content)| {
                                    resolve_ws.send(conflict.resolve_message(
                                        action,
                                        result_content,
                                        current_scope_nonce.get_untracked(),
                                    ));
                                    set_diff_content.set(None);
                                })
                            });
                            view! {
                                <crate::components::diff_view::DiffView
                                    session=session
                                    is_readonly=is_spectator.get()
                                    force_unified=true
                                    mobile=true
                                    on_compute_projection=Some(on_compute_projection)
                                    on_persist_draft=Some(on_persist_draft)
                                    on_resolve_merge_conflict=on_resolve
                                    on_close=Callback::new(move |_| set_diff_content.set(None))
                                />
                            }
                            .into_any()
                    } else if let Some(doc_id) = current_editor_doc.get() {
                        view! { <Editor doc_id=doc_id on_stats=on_stats embedded=true /> }.into_any()
                    } else {
                        view! { <Dashboard /> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MobileContentSurface, mobile_content_keyboard_overlay_style,
        mobile_content_surface_after_diff_close,
    };

    #[test]
    fn mobile_toolbar_keyboard_native_overlay_shrinks_editor_presentation_only() {
        assert_eq!(
            mobile_content_keyboard_overlay_style(338),
            "padding-bottom: 338px;"
        );
        assert_eq!(
            mobile_content_keyboard_overlay_style(-1),
            "padding-bottom: 0px;"
        );
    }

    #[test]
    fn mobile_diff_close_returns_to_editor_surface() {
        assert_eq!(
            mobile_content_surface_after_diff_close(true, false, false),
            MobileContentSurface::Editor
        );
    }

    #[test]
    fn mobile_diff_close_without_current_doc_returns_dashboard() {
        assert_eq!(
            mobile_content_surface_after_diff_close(false, false, false),
            MobileContentSurface::Dashboard
        );
    }

    #[test]
    fn mobile_diff_close_respects_pending_switch_gates() {
        assert_eq!(
            mobile_content_surface_after_diff_close(true, true, false),
            MobileContentSurface::Dashboard
        );
        assert_eq!(
            mobile_content_surface_after_diff_close(true, false, true),
            MobileContentSurface::Dashboard
        );
    }
}
