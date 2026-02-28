// apps\web\src\components\search_box
use crate::components::search_box::SearchUiMode;
use crate::components::search_box::sheet_gesture;
use crate::components::search_box::types::SearchResult;
use crate::components::search_box::ui_footer::footer;
use crate::components::search_box::ui_sections;
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

    let panel_class = move || match ui_mode.get() {
        SearchUiMode::Sheet => {
            "absolute top-0 left-0 right-0 bg-panel rounded-b-2xl shadow-xl border border-default overflow-hidden flex flex-col max-h-[72vh] animate-in fade-in slide-in-from-top-4 duration-200 ease-out"
        }
        SearchUiMode::Overlay => {
            "absolute top-14 left-1/2 -translate-x-1/2 w-full max-w-xl bg-panel rounded-lg shadow-xl border border-default overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-200 ease-out"
        }
    };
    let panel_style = move || match ui_mode.get() {
        SearchUiMode::Sheet => {
            let y = sheet_drag_offset.get();
            let transition = if sheet_dragging.get() {
                "none"
            } else {
                "transform 200ms ease-out"
            };
            format!(
                "padding-top: env(safe-area-inset-top); transform: translateY({}px); transition: {};",
                y, transition
            )
        }
        SearchUiMode::Overlay => "".to_string(),
    };
    let backdrop_class = move || match ui_mode.get() {
        SearchUiMode::Sheet => "fixed inset-0 z-[100] font-sans bg-black/20 backdrop-blur-[1px]",
        SearchUiMode::Overlay => "fixed inset-0 z-[100] font-sans",
    };

    view! {
        <Show when=move || show.get()>
            <div
                class=backdrop_class
                on:click=move |_| set_show.set(false)
            >
                <div
                    class=panel_class
                    style=panel_style
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:touchstart=move |ev: TouchEvent| {
                        if ui_mode.get_untracked() == SearchUiMode::Sheet {
                            sheet_gesture::on_start(
                                &ev,
                                &results_ref,
                                set_touch_start_x,
                                set_touch_start_y,
                                set_touch_start_at,
                                set_can_dismiss_sheet,
                            );
                            set_sheet_dragging.set(true);
                        }
                    }
                    on:touchmove=move |ev: TouchEvent| {
                        if ui_mode.get_untracked() == SearchUiMode::Sheet {
                            let start_y = touch_start_y.get_untracked();
                            if let Some(touch) = ev.changed_touches().get(0) {
                                let offset = sheet_gesture::damped_offset(
                                    start_y,
                                    touch.client_y(),
                                    can_dismiss_sheet.get_untracked(),
                                );
                                set_sheet_drag_offset.set(offset);
                            }
                        }
                    }
                    on:touchend=move |ev: TouchEvent| {
                        if ui_mode.get_untracked() == SearchUiMode::Sheet {
                            if sheet_gesture::should_close(
                                &ev,
                                touch_start_x,
                                touch_start_y,
                                touch_start_at,
                                can_dismiss_sheet,
                            ) {
                                set_show.set(false);
                            }
                            set_sheet_dragging.set(false);
                            set_sheet_drag_offset.set(0);
                            sheet_gesture::reset(set_can_dismiss_sheet);
                        }
                    }
                    on:touchcancel=move |_| {
                        set_sheet_dragging.set(false);
                        set_sheet_drag_offset.set(0);
                        sheet_gesture::reset(set_can_dismiss_sheet)
                    }
                    on:keydown={
                        let handle_keydown_closure = handle_keydown_closure.clone();
                        move |ev| handle_keydown_closure(ev)
                    }
                >
                    {move || if ui_mode.get() == SearchUiMode::Sheet {
                        view! {
                            <div data-sheet-drag-handle="1" class="flex justify-center py-2">
                                <div class="w-10 h-1.5 rounded-full bg-active"></div>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {}.into_any()
                    }}
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
