// apps\web\src\components\search_box
//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use crate::components::search_box::SearchUiMode;
use crate::components::search_box::types::SearchResult;
use crate::components::search_box::ui_footer::footer;
use crate::components::search_box::ui_sections;
use crate::components::search_box::ui_sheet;
use crate::hooks::use_core::CoreState;
use crate::i18n::Locale;
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, MouseEvent, TouchEvent};
/// 负责渲染整体遮罩与内部布局。
#[allow(clippy::too_many_arguments)]
pub fn render_overlay(
    show: Signal<bool>,
    set_show: WriteSignal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    placeholder_text: Memo<String>,
    handle_keydown: Arc<dyn Fn(KeyboardEvent) + Send + Sync>,
    providers_results: Memo<Vec<SearchResult>>,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    input_ref: NodeRef<leptos::html::Input>,
    core: CoreState,
    locale: RwSignal<Locale>,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
    ui_mode: Signal<SearchUiMode>,
) -> impl IntoView {
    let handle_keydown_closure = handle_keydown.clone();
    let active_index_closure = active_index.clone();
    let results_ref = NodeRef::<leptos::html::Div>::new();
    let (touch_start_x, set_touch_start_x) = signal(0i32);
    let (touch_start_y, set_touch_start_y) = signal(0i32);
    let (touch_start_at, set_touch_start_at) = signal(0.0f64);
    let (can_dismiss_sheet, set_can_dismiss_sheet) = signal(false);
    let (sheet_drag_offset, set_sheet_drag_offset) = signal(0i32);
    let (sheet_dragging, set_sheet_dragging) = signal(false);

    view! {
        <Show when=move || show.get()>
            <div
                class=move || ui_sheet::backdrop_class(ui_mode.get())
                on:click=move |_| set_show.set(false)
            >
                <div
                    class=move || ui_sheet::panel_class(ui_mode.get())
                    style=move || ui_sheet::panel_style(
                        ui_mode.get(),
                        sheet_drag_offset.get(),
                        sheet_dragging.get(),
                    )
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:touchstart=move |ev: TouchEvent| {
                        ui_sheet::handle_touch_start(
                            ev,
                            ui_mode,
                            &results_ref,
                            set_touch_start_x,
                            set_touch_start_y,
                            set_touch_start_at,
                            set_can_dismiss_sheet,
                            set_sheet_dragging,
                        );
                    }
                    on:touchmove=move |ev: TouchEvent| {
                        ui_sheet::handle_touch_move(
                            ev,
                            ui_mode,
                            touch_start_y,
                            can_dismiss_sheet,
                            set_sheet_drag_offset,
                        );
                    }
                    on:touchend=move |ev: TouchEvent| {
                        ui_sheet::handle_touch_end(
                            ev,
                            ui_mode,
                            touch_start_x,
                            touch_start_y,
                            touch_start_at,
                            can_dismiss_sheet,
                            set_show,
                            set_sheet_dragging,
                            set_sheet_drag_offset,
                            set_can_dismiss_sheet,
                        );
                    }
                    on:touchcancel=move |_| ui_sheet::handle_touch_cancel(
                        set_sheet_dragging,
                        set_sheet_drag_offset,
                        set_can_dismiss_sheet,
                    )
                    on:keydown={
                        let handle_keydown_closure = handle_keydown_closure.clone();
                        move |ev| handle_keydown_closure(ev)
                    }
                >
                    {move || ui_sheet::drag_handle(ui_mode.get()).into_any()}
                    {ui_sections::header(
                        query,
                        set_query,
                        set_selected_index,
                        placeholder_text,
                        input_ref,
                        ui_mode,
                    )}
                    {ui_sections::results_panel(
                        providers_results,
                        selected_index,
                        set_selected_index,
                        active_index_closure.clone(),
                        set_show,
                        set_query,
                        input_ref,
                        core.clone(),
                        locale,
                        set_recent_move_dirs,
                        results_ref,
                    )}
                    {footer(ui_mode)}
                </div>
            </div>
        </Show>
    }
}
