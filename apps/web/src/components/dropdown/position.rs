//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use super::{Align, AnchorRect};

pub(super) struct DropdownPlacement {
    pub open_up: bool,
    pub max_height: Option<f64>,
}

pub(super) fn measure_dropdown(
    el: &web_sys::Element,
    anchor: AnchorRect,
    offset: f64,
) -> DropdownPlacement {
    let rect = el.get_bounding_client_rect();
    let height = rect.height();
    let viewport = viewport_height();
    let space_below = (viewport - anchor.bottom - offset).max(0.0);
    let space_above = (anchor.top - offset).max(0.0);

    if space_below < height && space_above >= height {
        DropdownPlacement {
            open_up: true,
            max_height: None,
        }
    } else if space_below < height && space_above < height {
        DropdownPlacement {
            open_up: true,
            max_height: Some(space_above.max(120.0)),
        }
    } else {
        DropdownPlacement {
            open_up: false,
            max_height: None,
        }
    }
}

pub(super) fn build_panel_style(
    anchor: AnchorRect,
    align: Align,
    offset: f64,
    open_up: bool,
    max_height: Option<f64>,
    ready: bool,
) -> String {
    let mut style = String::new();
    let viewport = viewport_height();

    match align {
        Align::Left => style.push_str(&format!("left: {}px;", anchor.left)),
        Align::Right => {
            style.push_str(&format!("left: {}px;", anchor.right));
            style.push_str("transform: translateX(-100%);");
        }
    }

    if open_up {
        let bottom = (viewport - anchor.top + offset).max(0.0);
        style.push_str(&format!("bottom: {}px;", bottom));
    } else {
        style.push_str(&format!("top: {}px;", anchor.bottom + offset));
    }

    if let Some(max_h) = max_height {
        style.push_str(&format!("max-height: {}px; overflow-y: auto;", max_h));
    }

    if !ready {
        style.push_str("visibility: hidden;");
    }

    style
}

fn viewport_height() -> f64 {
    web_sys::window()
        .expect("window")
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}
