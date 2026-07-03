//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!
//! Shared viewport breakpoint helpers for UI shell routing.

pub(crate) const MOBILE_BREAKPOINT_WIDTH: f64 = 768.0;

pub(crate) fn viewport_width_maps_to_mobile(width: f64) -> bool {
    width <= MOBILE_BREAKPOINT_WIDTH
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn current_viewport_width() -> Option<f64> {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn current_viewport_width() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::{MOBILE_BREAKPOINT_WIDTH, current_viewport_width, viewport_width_maps_to_mobile};

    #[test]
    fn mobile_viewport_mapping_uses_inclusive_768px_boundary() {
        assert!(viewport_width_maps_to_mobile(375.0));
        assert!(viewport_width_maps_to_mobile(MOBILE_BREAKPOINT_WIDTH));
        assert!(!viewport_width_maps_to_mobile(
            MOBILE_BREAKPOINT_WIDTH + 0.1
        ));
        assert!(!viewport_width_maps_to_mobile(1024.0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn current_viewport_width_is_unavailable_without_browser_window() {
        assert!(current_viewport_width().is_none());
    }
}
