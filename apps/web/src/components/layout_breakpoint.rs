//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!
//! Shared viewport breakpoint helpers for UI shell routing.

pub(crate) const MOBILE_BREAKPOINT_WIDTH: f64 = 768.0;

pub(crate) fn viewport_width_maps_to_mobile(width: f64) -> bool {
    width <= MOBILE_BREAKPOINT_WIDTH
}

pub(crate) fn mobile_command_surface_matches(viewport_mobile: bool, touch_mobile: bool) -> bool {
    viewport_mobile || touch_mobile
}

pub(crate) fn current_command_surface_maps_to_mobile() -> bool {
    mobile_command_surface_matches(
        current_viewport_width().is_some_and(viewport_width_maps_to_mobile),
        current_touch_input_maps_to_mobile(),
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn current_viewport_width() -> Option<f64> {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
}

#[cfg(target_arch = "wasm32")]
fn current_touch_input_maps_to_mobile() -> bool {
    web_sys::window()
        .and_then(|window| {
            window
                .match_media("(hover: none), (pointer: coarse)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn current_viewport_width() -> Option<f64> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn current_touch_input_maps_to_mobile() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::{
        MOBILE_BREAKPOINT_WIDTH, current_command_surface_maps_to_mobile, current_viewport_width,
        mobile_command_surface_matches, viewport_width_maps_to_mobile,
    };

    #[test]
    fn mobile_viewport_mapping_uses_inclusive_768px_boundary() {
        assert!(viewport_width_maps_to_mobile(375.0));
        assert!(viewport_width_maps_to_mobile(MOBILE_BREAKPOINT_WIDTH));
        assert!(!viewport_width_maps_to_mobile(
            MOBILE_BREAKPOINT_WIDTH + 0.1
        ));
        assert!(!viewport_width_maps_to_mobile(1024.0));
    }

    #[test]
    fn mobile_command_surface_accepts_viewport_or_touch_input() {
        assert!(mobile_command_surface_matches(true, false));
        assert!(mobile_command_surface_matches(false, true));
        assert!(mobile_command_surface_matches(true, true));
        assert!(!mobile_command_surface_matches(false, false));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn current_viewport_width_is_unavailable_without_browser_window() {
        assert!(current_viewport_width().is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn current_command_surface_is_desktop_without_browser_window() {
        assert!(!current_command_surface_maps_to_mobile());
    }
}
