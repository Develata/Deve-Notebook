pub mod header;
mod line_render;
pub mod model;
mod navigation;
mod split_pane;
mod state;
pub mod unified;
mod unified_pane;

use self::header::DiffHeader;
use self::navigation::{create_hunk_nav, should_ignore_shortcut};
use self::split_pane::SplitPane;
use self::state::{create_compute_state, create_unified_viewport};
use self::unified_pane::UnifiedPane;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn DiffView(
    path: String,
    old_content: String,
    new_content: String,
    #[prop(default = false)] is_readonly: bool,
    #[prop(default = false)] force_unified: bool,
    #[prop(default = false)] mobile: bool,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let filename = path
        .replace('\\', "/")
        .split('/')
        .next_back()
        .unwrap_or("?")
        .to_string();

    let compute = create_compute_state(old_content, new_content);
    let left_ref = NodeRef::<html::Div>::new();
    let right_ref = NodeRef::<html::Div>::new();
    let (syncing_left, set_syncing_left) = signal(false);
    let (syncing_right, set_syncing_right) = signal(false);
    let viewport = create_unified_viewport(compute.unified_lines);
    let nav = create_hunk_nav(
        compute.unified_lines,
        force_unified,
        viewport.unified_ref,
        left_ref,
        right_ref,
    );

    let on_close_for_key = on_close;
    let on_prev_hunk_for_key = nav.on_prev_hunk;
    let on_next_hunk_for_key = nav.on_next_hunk;
    let _esc_listener =
        window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
            if should_ignore_shortcut(&ev) {
                return;
            }
            if ev.key() == "Escape" {
                ev.prevent_default();
                on_close_for_key.run(());
                return;
            }
            let key = ev.key();
            if key == "]" || (ev.alt_key() && key == "ArrowDown") {
                ev.prevent_default();
                on_next_hunk_for_key.run(());
                return;
            }
            if key == "[" || (ev.alt_key() && key == "ArrowUp") {
                ev.prevent_default();
                on_prev_hunk_for_key.run(());
            }
        });

    view! {
        <div class=move || if mobile { "diff-view-mobile h-full w-full bg-[var(--diff-bg)] flex flex-col font-mono text-[13px]" } else { "h-full w-full bg-[var(--diff-bg)] flex flex-col font-mono text-[13px]" }>
            <DiffHeader mobile=mobile filename=filename is_readonly=is_readonly is_editing=compute.is_editing hunk_index_text=nav.hunk_index_text has_hunks=nav.has_hunks added_count=nav.added_count deleted_count=nav.deleted_count on_prev_hunk=nav.on_prev_hunk on_next_hunk=nav.on_next_hunk toggle_edit=Callback::new(move |_| compute.set_is_editing.update(|v| *v = !*v)) on_close=on_close />
            <div class="flex-1 overflow-hidden flex relative">
                <Show when=move || compute.compute_state.get() == "computing">
                    <div class="diff-compute-indicator absolute top-2 right-2 z-10 rounded border border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-2 py-1 text-[11px] text-[var(--diff-muted)] shadow-sm">
                        {move || t::diff::computing(locale.get())}
                    </div>
                </Show>
                {move || if force_unified {
                    if compute.is_editing.get() {
                        view! { <div class="flex-1 overflow-auto"><textarea name="diff-edit-mobile" class="w-full h-full p-2 resize-none outline-none font-mono text-[13px] bg-[var(--diff-bg)] text-[var(--diff-fg)] border-none" prop:value=move || compute.content.get() on:input=move |ev| compute.set_content.set(event_target_value(&ev))></textarea></div> }.into_any()
                    } else {
                        view! { <UnifiedPane lines=compute.unified_lines visible_lines=viewport.visible_unified window=viewport.window unified_ref=viewport.unified_ref set_scroll_top=viewport.set_scroll_top set_viewport_h=viewport.set_viewport_h compute_state=compute.compute_state /> }.into_any()
                    }
                } else {
                    view! { <SplitPane diff_result=compute.diff_result left_ref=left_ref right_ref=right_ref syncing_left=syncing_left set_syncing_left=set_syncing_left syncing_right=syncing_right set_syncing_right=set_syncing_right is_editing=compute.is_editing content=compute.content set_content=compute.set_content /> }.into_any()
                }}
            </div>
        </div>
    }
}
