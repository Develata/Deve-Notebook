use crate::components::search_box::SearchUiMode;
use crate::components::search_box::sheet_gesture;
use leptos::html;
use leptos::prelude::*;
use web_sys::TouchEvent;

#[path = "ui_sheet_style.rs"]
mod ui_sheet_style;

pub(super) fn panel_class(mode: SearchUiMode) -> &'static str {
    ui_sheet_style::panel_class(mode)
}

pub(super) fn panel_style(
    mode: SearchUiMode,
    sheet_drag_offset: i32,
    sheet_dragging: bool,
) -> String {
    ui_sheet_style::panel_style(mode, sheet_drag_offset, sheet_dragging)
}

pub(super) fn backdrop_class(mode: SearchUiMode) -> &'static str {
    ui_sheet_style::backdrop_class(mode)
}

pub(super) fn drag_handle(mode: SearchUiMode) -> impl IntoView {
    ui_sheet_style::drag_handle(mode)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_touch_start(
    ev: TouchEvent,
    ui_mode: Signal<SearchUiMode>,
    results_ref: &NodeRef<html::Div>,
    set_touch_start_x: WriteSignal<i32>,
    set_touch_start_y: WriteSignal<i32>,
    set_touch_start_at: WriteSignal<f64>,
    set_can_dismiss_sheet: WriteSignal<bool>,
    set_sheet_dragging: WriteSignal<bool>,
) {
    if ui_mode.get_untracked() == SearchUiMode::Sheet {
        sheet_gesture::on_start(
            &ev,
            results_ref,
            set_touch_start_x,
            set_touch_start_y,
            set_touch_start_at,
            set_can_dismiss_sheet,
        );
        set_sheet_dragging.set(true);
    }
}

pub(super) fn handle_touch_move(
    ev: TouchEvent,
    ui_mode: Signal<SearchUiMode>,
    touch_start_y: ReadSignal<i32>,
    can_dismiss_sheet: ReadSignal<bool>,
    set_sheet_drag_offset: WriteSignal<i32>,
) {
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

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_touch_end(
    ev: TouchEvent,
    ui_mode: Signal<SearchUiMode>,
    touch_start_x: ReadSignal<i32>,
    touch_start_y: ReadSignal<i32>,
    touch_start_at: ReadSignal<f64>,
    can_dismiss_sheet: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
    set_sheet_dragging: WriteSignal<bool>,
    set_sheet_drag_offset: WriteSignal<i32>,
    set_can_dismiss_sheet: WriteSignal<bool>,
) {
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
        handle_touch_cancel(
            set_sheet_dragging,
            set_sheet_drag_offset,
            set_can_dismiss_sheet,
        );
    }
}

pub(super) fn handle_touch_cancel(
    set_sheet_dragging: WriteSignal<bool>,
    set_sheet_drag_offset: WriteSignal<i32>,
    set_can_dismiss_sheet: WriteSignal<bool>,
) {
    set_sheet_dragging.set(false);
    set_sheet_drag_offset.set(0);
    sheet_gesture::reset(set_can_dismiss_sheet);
}
