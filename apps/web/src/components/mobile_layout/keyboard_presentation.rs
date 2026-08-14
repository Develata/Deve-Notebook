//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! Keyboard presentation is projection-only. It separates keyboard visibility
//! from the extra bottom offset needed by an overlay presentation.

use super::native_presentation::NativeImePresentation;

const VIEWPORT_RESIZE_EPSILON_CSS_PX: f64 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum KeyboardPresentationSource {
    #[default]
    Hidden,
    VisualViewport,
    NativeInsets,
}

impl KeyboardPresentationSource {
    pub(super) fn as_dom_value(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::VisualViewport => "visual-viewport",
            Self::NativeInsets => "native-insets",
        }
    }

    pub(super) fn is_visible(self) -> bool {
        self != Self::Hidden
    }

    pub(super) fn uses_native_overlay(self) -> bool {
        self == Self::NativeInsets
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct KeyboardPresentation {
    pub(super) offset: i32,
    pub(super) source: KeyboardPresentationSource,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ViewportObservation {
    pub(super) width: i32,
    pub(super) inner_height: f64,
    pub(super) viewport_height: f64,
    pub(super) offset_top: f64,
}

impl ViewportObservation {
    fn is_valid(self) -> bool {
        self.width > 0
            && self.inner_height.is_finite()
            && self.viewport_height.is_finite()
            && self.offset_top.is_finite()
            && self.inner_height > 0.0
            && self.viewport_height > 0.0
            && self.offset_top >= 0.0
    }
}

#[derive(Clone, Copy, Debug)]
struct HiddenViewportBaseline {
    generation: u64,
    width: i32,
    viewport_height: f64,
}

#[derive(Default)]
pub(super) struct KeyboardPresentationResolver {
    hidden_baseline: Option<HiddenViewportBaseline>,
}

impl KeyboardPresentationResolver {
    pub(super) fn resolve(
        &mut self,
        viewport: Option<ViewportObservation>,
        native_ime: Option<NativeImePresentation>,
    ) -> KeyboardPresentation {
        let Some(viewport) = viewport.filter(|value| value.is_valid()) else {
            return KeyboardPresentation::default();
        };
        if let Some(native) = native_ime
            && !native.is_visible()
        {
            self.observe_hidden_viewport(viewport, native.generation());
        }

        let visual_offset = visual_viewport_keyboard_offset(
            viewport.inner_height,
            viewport.viewport_height,
            viewport.offset_top,
        );
        if visual_offset > 0 {
            return KeyboardPresentation {
                offset: visual_offset,
                source: KeyboardPresentationSource::VisualViewport,
            };
        }

        let Some(native) = native_ime.filter(|value| value.is_visible()) else {
            return KeyboardPresentation::default();
        };
        let Some(baseline) = self.hidden_baseline.filter(|baseline| {
            baseline.generation == native.generation() && baseline.width == viewport.width
        }) else {
            return KeyboardPresentation::default();
        };
        if baseline.viewport_height - viewport.viewport_height > VIEWPORT_RESIZE_EPSILON_CSS_PX {
            return KeyboardPresentation {
                offset: 0,
                source: KeyboardPresentationSource::VisualViewport,
            };
        }

        let native_offset = native.usable_offset();
        if native_offset > 0 {
            KeyboardPresentation {
                offset: native_offset,
                source: KeyboardPresentationSource::NativeInsets,
            }
        } else {
            KeyboardPresentation::default()
        }
    }

    fn observe_hidden_viewport(&mut self, viewport: ViewportObservation, generation: u64) {
        // Native `imeVisible = false` is the platform's settled-hidden admission
        // signal. Keep the most recent admitted geometry rather than a historic
        // maximum so same-width split-screen/window changes retire old facts.
        self.hidden_baseline = Some(HiddenViewportBaseline {
            generation,
            width: viewport.width,
            viewport_height: viewport.viewport_height,
        });
    }
}

pub(super) fn visual_viewport_keyboard_offset(
    inner_height: f64,
    viewport_height: f64,
    offset_top: f64,
) -> i32 {
    if viewport_height <= 0.0 || inner_height <= 0.0 {
        return 0;
    }
    (inner_height - (viewport_height + offset_top))
        .max(0.0)
        .round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport(width: i32, inner_height: f64, viewport_height: f64) -> ViewportObservation {
        ViewportObservation {
            width,
            inner_height,
            viewport_height,
            offset_top: 0.0,
        }
    }

    fn hidden(generation: u64) -> NativeImePresentation {
        NativeImePresentation::from_generation_and_usable_offset_for_test(generation, 0)
    }

    fn visible(generation: u64, offset: i32) -> NativeImePresentation {
        NativeImePresentation::from_generation_and_usable_offset_for_test(generation, offset)
    }

    #[test]
    fn mobile_toolbar_keyboard_offset_uses_visual_viewport_overlap() {
        assert_eq!(visual_viewport_keyboard_offset(812.0, 500.0, 0.0), 312);
        assert_eq!(visual_viewport_keyboard_offset(812.0, 500.0, 12.4), 300);
    }

    #[test]
    fn mobile_toolbar_keyboard_offset_clamps_to_zero_without_overlap() {
        assert_eq!(visual_viewport_keyboard_offset(812.0, 812.0, 0.0), 0);
        assert_eq!(visual_viewport_keyboard_offset(812.0, 900.0, 0.0), 0);
        assert_eq!(visual_viewport_keyboard_offset(812.0, 0.0, 0.0), 0);
    }

    #[test]
    fn mobile_toolbar_keyboard_visual_viewport_precedes_native_ime_fallback() {
        let mut resolver = KeyboardPresentationResolver::default();
        resolver.resolve(Some(viewport(392, 812.0, 812.0)), Some(hidden(1)));
        assert_eq!(
            resolver.resolve(Some(viewport(392, 812.0, 500.0)), Some(visible(1, 338))),
            KeyboardPresentation {
                offset: 312,
                source: KeyboardPresentationSource::VisualViewport,
            }
        );
    }

    #[test]
    fn mobile_toolbar_keyboard_uses_current_native_ime_fallback_without_viewport_overlap() {
        let mut resolver = KeyboardPresentationResolver::default();
        resolver.resolve(Some(viewport(392, 812.0, 812.0)), Some(hidden(1)));
        assert_eq!(
            resolver.resolve(Some(viewport(392, 812.0, 812.0)), Some(visible(1, 338))),
            KeyboardPresentation {
                offset: 338,
                source: KeyboardPresentationSource::NativeInsets,
            }
        );
    }

    #[test]
    fn mobile_toolbar_keyboard_adjust_resize_never_reuses_root_inset_as_overlay() {
        let mut resolver = KeyboardPresentationResolver::default();
        resolver.resolve(Some(viewport(392, 872.0, 872.0)), Some(hidden(4)));
        assert_eq!(
            resolver.resolve(Some(viewport(392, 510.0, 510.0)), Some(visible(4, 338))),
            KeyboardPresentation {
                offset: 0,
                source: KeyboardPresentationSource::VisualViewport,
            }
        );
    }

    #[test]
    fn mobile_toolbar_keyboard_native_fallback_requires_same_width_generation_hidden_baseline() {
        let mut resolver = KeyboardPresentationResolver::default();
        resolver.resolve(Some(viewport(392, 872.0, 872.0)), Some(hidden(4)));
        assert_eq!(
            resolver.resolve(Some(viewport(393, 872.0, 872.0)), Some(visible(4, 338))),
            KeyboardPresentation::default()
        );
        assert_eq!(
            resolver.resolve(Some(viewport(392, 872.0, 872.0)), Some(visible(5, 338))),
            KeyboardPresentation::default()
        );
    }

    #[test]
    fn mobile_toolbar_keyboard_recent_hidden_viewport_retires_same_width_historic_maximum() {
        let mut resolver = KeyboardPresentationResolver::default();
        resolver.resolve(Some(viewport(392, 872.0, 872.0)), Some(hidden(4)));
        resolver.resolve(Some(viewport(392, 700.0, 700.0)), Some(hidden(4)));

        assert_eq!(
            resolver.resolve(Some(viewport(392, 700.0, 700.0)), Some(visible(4, 280))),
            KeyboardPresentation {
                offset: 280,
                source: KeyboardPresentationSource::NativeInsets,
            }
        );
    }
}
