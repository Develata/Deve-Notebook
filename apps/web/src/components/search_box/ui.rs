// apps\web\src\components\search_box
//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::focus_scope;
use crate::components::search_box::SearchUiMode;
use crate::components::search_box::runtime::SearchRuntime;
use crate::components::search_box::types::SearchResult;
use crate::components::search_box::ui_footer::footer;
use crate::components::search_box::ui_sections;
use crate::components::search_box::ui_sheet;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, MouseEvent, TouchEvent};

pub struct SearchOverlayView {
    pub show: Signal<bool>,
    pub set_show: WriteSignal<bool>,
    pub query: Signal<String>,
    pub set_query: WriteSignal<String>,
    pub placeholder_text: Memo<String>,
    pub handle_keydown: Arc<dyn Fn(KeyboardEvent) + Send + Sync>,
    pub providers_results: Memo<Vec<SearchResult>>,
    pub selected_index: Signal<usize>,
    pub set_selected_index: WriteSignal<usize>,
    pub active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    pub input_ref: NodeRef<leptos::html::Input>,
    pub runtime: SearchRuntime,
    pub locale: RwSignal<Locale>,
    pub set_recent_move_dirs: WriteSignal<Vec<String>>,
    pub ui_mode: Signal<SearchUiMode>,
}

pub(super) fn search_dialog_label(locale: Locale, query: &str) -> &'static str {
    match super::logic::search_surface_mode(query) {
        super::logic::SearchSurfaceMode::Command | super::logic::SearchSurfaceMode::FileOp => {
            t::header::command(locale)
        }
        _ => t::sidebar::search(locale),
    }
}

/// 负责渲染整体遮罩与内部布局。
pub fn render_overlay(view: SearchOverlayView) -> impl IntoView {
    let SearchOverlayView {
        show,
        set_show,
        query,
        set_query,
        placeholder_text,
        handle_keydown,
        providers_results,
        selected_index,
        set_selected_index,
        active_index,
        input_ref,
        runtime,
        locale,
        set_recent_move_dirs,
        ui_mode,
    } = view;
    let handle_keydown_closure = handle_keydown.clone();
    let active_index_closure = active_index.clone();
    let results_ref = NodeRef::<leptos::html::Div>::new();
    let panel_ref = NodeRef::<leptos::html::Div>::new();
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
                    node_ref=panel_ref
                    role="dialog"
                    aria-modal="true"
                    aria-label=move || search_dialog_label(locale.get(), &query.get())
                    tabindex="-1"
                    data-deve-search-sheet-position=move || ui_sheet::sheet_position(ui_mode.get())
                    class=move || ui_sheet::panel_class(ui_mode.get())
                    style=move || ui_sheet::panel_style(
                        ui_mode.get(),
                        sheet_drag_offset.get(),
                        sheet_dragging.get(),
                    )
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:touchstart=move |ev: TouchEvent| {
                        ui_sheet::handle_touch_start(ui_sheet::SheetTouchStart {
                            ev,
                            ui_mode,
                            results_ref: &results_ref,
                            set_touch_start_x,
                            set_touch_start_y,
                            set_touch_start_at,
                            set_can_dismiss_sheet,
                            set_sheet_dragging,
                        });
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
                        ui_sheet::handle_touch_end(ui_sheet::SheetTouchEnd {
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
                        });
                    }
                    on:touchcancel=move |_| ui_sheet::handle_touch_cancel(
                        set_sheet_dragging,
                        set_sheet_drag_offset,
                        set_can_dismiss_sheet,
                    )
                    on:keydown={
                        let handle_keydown_closure = handle_keydown_closure.clone();
                        move |ev| {
                            if focus_scope::handle_focus_trap_keydown(&ev, panel_ref) {
                                return;
                            }
                            handle_keydown_closure(ev);
                        }
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
                    {ui_sections::results_panel(ui_sections::SearchResultsPanelView {
                        providers_results,
                        selected_index,
                        set_selected_index,
                        active_index: active_index_closure.clone(),
                        set_show,
                        set_query,
                        input_ref,
                        runtime: runtime.clone(),
                        locale,
                        set_recent_move_dirs,
                        results_ref,
                        ui_mode,
                    })}
                    {footer(ui_mode, locale)}
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::search_dialog_label;
    use crate::i18n::{Locale, t};

    #[test]
    fn mobile_search_sheet_dialog_named_bound() {
        assert_eq!(search_dialog_label(Locale::En, ""), "Search");
        assert_eq!(
            search_dialog_label(Locale::Zh, "?content"),
            t::sidebar::search(Locale::Zh)
        );
        assert_eq!(
            search_dialog_label(Locale::En, ">git status"),
            "Command Palette"
        );
        assert_eq!(
            search_dialog_label(Locale::Zh, ">git status"),
            t::header::command(Locale::Zh)
        );
    }
}
