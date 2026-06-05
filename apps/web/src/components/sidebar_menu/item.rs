//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
use crate::components::icons;
use crate::context_action::{
    ContextActionIcon, ContextActionId, ContextActionProjectionRequest, ContextActionSurface,
    ContextActionTarget, project_context_actions,
};
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub(super) fn SidebarMenuItems(
    locale: RwSignal<Locale>,
    is_readonly: Signal<bool>,
    target: ContextActionTarget,
    on_action: Callback<ContextActionId>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        {move || project_context_actions(ContextActionProjectionRequest::new(
                ContextActionSurface::FileTree,
                target.clone(),
                is_readonly.get(),
            ))
            .into_iter()
            .map(|action| {
                let action_id = action.id;
                let is_danger = action.is_destructive();
                let icon_cls = format!(
                    "w-4 h-4 {}",
                    if is_danger {
                        "text-red-500 group-hover:text-red-600"
                    } else {
                        "text-muted"
                    }
                );

                view! {
                    <>
                        {if action.separator_before {
                            Some(view! { <div class="my-1 border-t border-default"></div> })
                        } else {
                            None
                        }}
                        <button
                            class=format!(
                                "w-full text-left px-3 py-1.5 hover:bg-hover flex items-center gap-2 {}",
                                if is_danger { "text-red-600 group" } else { "" }
                            )
                            on:click=move |_| {
                                leptos::logging::log!(
                                    "SidebarMenu: Button clicked, action_id={}",
                                    action.stable_id(),
                                );
                                on_action.run(action_id);
                                on_close.run(());
                            }
                        >
                            {menu_icon(action.icon, &icon_cls)}
                            {move || action.label(locale.get())}
                            {if action.shows_external_provenance() {
                                Some(view! { <icons::ExternalLink class="ml-auto h-3 w-3 text-muted"/> })
                            } else {
                                None
                            }}
                        </button>
                    </>
                }
            })
            .collect_view()}
    }
}

fn menu_icon(icon: ContextActionIcon, class: &str) -> AnyView {
    let cls = class.to_string();
    match icon {
        ContextActionIcon::Rename => view! { <icons::Pencil class=cls/> }.into_any(),
        ContextActionIcon::Copy => view! { <icons::Copy class=cls/> }.into_any(),
        ContextActionIcon::OpenInNewWindow => view! { <icons::ExternalLink class=cls/> }.into_any(),
        ContextActionIcon::MoveTo => view! { <icons::FolderInput class=cls/> }.into_any(),
        ContextActionIcon::Delete => view! { <icons::Trash2 class=cls/> }.into_any(),
        ContextActionIcon::ExportPdf => view! { <icons::Download class=cls/> }.into_any(),
    }
}
