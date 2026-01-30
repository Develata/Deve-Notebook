// apps/web/src/components/dropdown
//! # Dropdown 组件 (自动翻转)
//!
//! 根据触发器位置与内容高度，自动向上/向下展开。

use leptos::prelude::*;

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
            let rect = el.get_bounding_client_rect();
            let height = rect.height();
            let window = web_sys::window().expect("window");
            let viewport = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let space_below = (viewport - anchor.bottom - offset).max(0.0);
            let space_above = (anchor.top - offset).max(0.0);
            if space_below < height && space_above >= height {
                set_open_up.set(true);
                set_max_height.set(None);
            } else if space_below < height && space_above < height {
                set_open_up.set(true);
                set_max_height.set(Some(space_above.max(120.0)));
            } else {
                set_open_up.set(false);
                set_max_height.set(None);
            }
            set_ready.set(true);
        });
    });

    let panel_style = Signal::derive(move || {
        let Some(anchor) = anchor.get() else {
            return "display: none;".to_string();
        };
        let mut style = String::new();
        let window = web_sys::window().expect("window");
        let viewport = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        match align {
            Align::Left => style.push_str(&format!("left: {}px;", anchor.left)),
            Align::Right => {
                style.push_str(&format!("left: {}px;", anchor.right));
                style.push_str("transform: translateX(-100%);");
            }
        }

        if open_up.get() {
            let bottom = (viewport - anchor.top + offset).max(0.0);
            style.push_str(&format!("bottom: {}px;", bottom));
        } else {
            style.push_str(&format!("top: {}px;", anchor.bottom + offset));
        }

        if let Some(max_h) = max_height.get() {
            style.push_str(&format!("max-height: {}px; overflow-y: auto;", max_h));
        }

        if !ready.get() {
            style.push_str("visibility: hidden;");
        }

        style
    });

    view! {
        <>
            <div
                class="fixed inset-0 z-40"
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_close.run(());
                }
            ></div>
            <div
                node_ref=panel_ref
                class="fixed z-50"
                style=move || panel_style.get()
                on:click=move |ev| ev.stop_propagation()
            >
                {children()}
            </div>
        </>
    }
}
