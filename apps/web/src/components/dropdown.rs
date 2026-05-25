// apps/web/src/components/dropdown
//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # Dropdown 组件 (自动翻转)
//!
//! 根据触发器位置与内容高度，自动向上/向下展开。

use leptos::prelude::*;

mod position;

use position::{build_panel_style, measure_dropdown};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Align {
    Left,
    #[default]
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorRect {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

#[component]
pub fn Dropdown(
    anchor: Signal<Option<AnchorRect>>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional)] align: Align,
    #[prop(optional, default = 6.0)] offset: f64,
    children: Children,
) -> impl IntoView {
    let align = if align == Align::Left {
        Align::Left
    } else {
        Align::Right
    };

    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let (open_up, set_open_up) = signal(false);
    let (max_height, set_max_height) = signal(None::<f64>);
    let (ready, set_ready) = signal(false);

    Effect::new(move |_| {
        if anchor.get().is_none() {
            set_ready.set(false);
            return;
        }
        request_animation_frame(move || {
            let Some(el) = panel_ref.get_untracked() else {
                return;
            };
            let Some(anchor) = anchor.get_untracked() else {
                return;
            };
            let placement = measure_dropdown(el.as_ref(), anchor, offset);
            set_open_up.set(placement.open_up);
            set_max_height.set(placement.max_height);
            set_ready.set(true);
        });
    });

    let panel_style = Signal::derive(move || {
        let Some(anchor) = anchor.get() else {
            return "display: none;".to_string();
        };
        build_panel_style(
            anchor,
            align,
            offset,
            open_up.get(),
            max_height.get(),
            ready.get(),
        )
    });

    view! {
        <>
            <div
                class="fixed inset-0 z-[var(--z-floating)]"
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_close.run(());
                }
            ></div>
            <div
                node_ref=panel_ref
                class="fixed z-[calc(var(--z-floating)_+_1)]"
                style=move || panel_style.get()
                on:click=move |ev| ev.stop_propagation()
            >
                {children()}
            </div>
        </>
    }
}
