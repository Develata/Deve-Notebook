//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use crate::components::search_box::SearchUiMode;
use crate::components::search_box::sheet_gesture;
use leptos::html;
use leptos::prelude::*;
use web_sys::TouchEvent;

mod style;

pub(super) fn panel_class(mode: SearchUiMode) -> &'static str {
    style::panel_class(mode)
}

pub(super) fn panel_style(
    mode: SearchUiMode,
    sheet_drag_offset: i32,
    sheet_dragging: bool,
) -> String {
    style::panel_style(mode, sheet_drag_offset, sheet_dragging)
}

pub(super) fn backdrop_class(mode: SearchUiMode) -> &'static str {
    style::backdrop_class(mode)
}

pub(super) fn sheet_position(mode: SearchUiMode) -> Option<&'static str> {
    style::sheet_position(mode)
}

pub(super) fn drag_handle(mode: SearchUiMode) -> impl IntoView {
    style::drag_handle(mode)
}

pub(super) struct SheetTouchStart<'a> {
    pub ev: TouchEvent,
    pub ui_mode: Signal<SearchUiMode>,
    pub results_ref: &'a NodeRef<html::Div>,
    pub set_touch_start_x: WriteSignal<i32>,
    pub set_touch_start_y: WriteSignal<i32>,
    pub set_touch_start_at: WriteSignal<f64>,
    pub set_can_dismiss_sheet: WriteSignal<bool>,
    pub set_sheet_dragging: WriteSignal<bool>,
}

pub(super) fn handle_touch_start(input: SheetTouchStart<'_>) {
    let SheetTouchStart {
        ev,
        ui_mode,
        results_ref,
        set_touch_start_x,
        set_touch_start_y,
        set_touch_start_at,
        set_can_dismiss_sheet,
        set_sheet_dragging,
    } = input;
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

pub(super) struct SheetTouchEnd {
    pub ev: TouchEvent,
    pub ui_mode: Signal<SearchUiMode>,
    pub touch_start_x: ReadSignal<i32>,
    pub touch_start_y: ReadSignal<i32>,
    pub touch_start_at: ReadSignal<f64>,
    pub can_dismiss_sheet: ReadSignal<bool>,
    pub set_show: WriteSignal<bool>,
    pub set_sheet_dragging: WriteSignal<bool>,
    pub set_sheet_drag_offset: WriteSignal<i32>,
    pub set_can_dismiss_sheet: WriteSignal<bool>,
}

pub(super) fn handle_touch_end(input: SheetTouchEnd) {
    let SheetTouchEnd {
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
    } = input;
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
