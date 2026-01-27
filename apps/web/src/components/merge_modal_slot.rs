// apps/web/src/components/merge_modal_slot.rs
use crate::components::merge_modal::MergeModal;
use leptos::prelude::*;

#[component]
pub fn MergeModalSlot() -> impl IntoView {
    let (show_merge, set_show_merge) = signal(false);
    provide_context(set_show_merge);

    view! {
        <MergeModal show=show_merge set_show=set_show_merge />
    }
}
