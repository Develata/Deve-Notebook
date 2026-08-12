//! plan_ref:
//!   - 11_ui_design/index#overlay-back-coordination
//!
//! Presentation-only back dispatch shared by Web surfaces and the Android shell.
//! The coordinator may close UI projections or dispatch an existing guarded
//! navigation callback; it never mutates repository authority.

use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;
use leptos::prelude::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
const NATIVE_BACK_REQUEST_EVENT: &str = "deve-native-back-request";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum UiBackLayer {
    Document = 100,
    Drawer = 200,
    TransientSheet = 300,
    Overlay = 400,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiBackOutcome {
    Handled,
    Unhandled,
}

impl UiBackOutcome {
    fn as_wire(self) -> &'static str {
        match self {
            Self::Handled => "Handled",
            Self::Unhandled => "Unhandled",
        }
    }
}

#[derive(Clone)]
pub(crate) struct UiBackCoordinator {
    handlers: Arc<Mutex<Vec<UiBackHandler>>>,
    next_id: Arc<AtomicU64>,
}

#[derive(Clone)]
struct UiBackHandler {
    id: u64,
    layer: UiBackLayer,
    handle: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl UiBackCoordinator {
    pub(crate) fn provide() -> Self {
        let coordinator = Self::detached();
        provide_context(coordinator.clone());
        coordinator
    }

    fn detached() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub(crate) fn register(
        &self,
        layer: UiBackLayer,
        handle: impl Fn() -> bool + Send + Sync + 'static,
    ) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.push(UiBackHandler {
                id,
                layer,
                handle: Arc::new(handle),
            });
        }

        let cleanup = self.clone();
        on_cleanup(move || cleanup.remove(id));
    }

    pub(crate) fn install_native_bridge(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let coordinator = self.clone();
        expect_context::<BrowserRuntimeLifetime>().register_event_listener(
            window.as_ref(),
            NATIVE_BACK_REQUEST_EVENT,
            move |event| coordinator.acknowledge_native_request(event),
        );
    }

    fn remove(&self, id: u64) {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.retain(|entry| entry.id != id);
        }
    }

    fn dispatch(&self) -> UiBackOutcome {
        let mut handlers = self
            .handlers
            .lock()
            .map(|handlers| handlers.clone())
            .unwrap_or_default();
        handlers.sort_by_key(|handler| (handler.layer, handler.id));
        if handlers.into_iter().rev().any(|handler| (handler.handle)()) {
            UiBackOutcome::Handled
        } else {
            UiBackOutcome::Unhandled
        }
    }

    fn acknowledge_native_request(&self, event: web_sys::Event) {
        let Ok(detail) = js_sys::Reflect::get(event.as_ref(), &"detail".into()) else {
            return;
        };
        let _ = js_sys::Reflect::set(&detail, &"listenerSeen".into(), &true.into());
        let request_id = js_sys::Reflect::get(&detail, &"requestId".into())
            .ok()
            .and_then(|value| value.as_string());
        if request_id.as_deref().is_none_or(str::is_empty) {
            return;
        }
        let _ = js_sys::Reflect::set(
            &detail,
            &"outcome".into(),
            &self.dispatch().as_wire().into(),
        );
    }
}

pub(crate) fn close_signal_projection(open: ReadSignal<bool>, set_open: WriteSignal<bool>) -> bool {
    if open.try_get_untracked() != Some(true) {
        return false;
    }
    set_open.set(false);
    true
}

#[cfg(test)]
mod tests {
    use super::{UiBackCoordinator, UiBackLayer, UiBackOutcome, close_signal_projection};
    use leptos::prelude::{GetUntracked, signal};
    use std::sync::{Arc, Mutex};

    #[test]
    fn ui_back_dispatches_highest_active_layer_then_falls_through() {
        let owner = leptos::prelude::Owner::new();
        owner.set();
        let coordinator = UiBackCoordinator::detached();
        let calls = Arc::new(Mutex::new(Vec::new()));

        let document_calls = calls.clone();
        coordinator.register(UiBackLayer::Document, move || {
            document_calls.lock().unwrap().push("document");
            true
        });
        let overlay_calls = calls.clone();
        coordinator.register(UiBackLayer::Overlay, move || {
            overlay_calls.lock().unwrap().push("overlay");
            false
        });
        let drawer_calls = calls.clone();
        coordinator.register(UiBackLayer::Drawer, move || {
            drawer_calls.lock().unwrap().push("drawer");
            true
        });

        assert_eq!(coordinator.dispatch(), UiBackOutcome::Handled);
        assert_eq!(*calls.lock().unwrap(), vec!["overlay", "drawer"]);
    }

    #[test]
    fn ui_back_is_unhandled_without_an_active_projection() {
        let coordinator = UiBackCoordinator::detached();
        assert_eq!(coordinator.dispatch(), UiBackOutcome::Unhandled);
    }

    #[test]
    fn ui_back_closes_latest_modal_or_menu_before_lower_layers() {
        let owner = leptos::prelude::Owner::new();
        owner.set();
        let coordinator = UiBackCoordinator::detached();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let drawer_open = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let menu_open = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let drawer_calls = calls.clone();
        let drawer_state = drawer_open.clone();
        coordinator.register(UiBackLayer::Drawer, move || {
            if !drawer_state.swap(false, std::sync::atomic::Ordering::AcqRel) {
                return false;
            }
            drawer_calls.lock().unwrap().push("drawer");
            true
        });
        let menu_calls = calls.clone();
        let menu_state = menu_open.clone();
        coordinator.register(UiBackLayer::Overlay, move || {
            if !menu_state.swap(false, std::sync::atomic::Ordering::AcqRel) {
                return false;
            }
            menu_calls.lock().unwrap().push("menu");
            true
        });

        assert_eq!(coordinator.dispatch(), UiBackOutcome::Handled);
        assert_eq!(*calls.lock().unwrap(), vec!["menu"]);
        assert!(drawer_open.load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(coordinator.dispatch(), UiBackOutcome::Handled);
        assert_eq!(*calls.lock().unwrap(), vec!["menu", "drawer"]);

        owner.cleanup();
        assert_eq!(coordinator.dispatch(), UiBackOutcome::Unhandled);
    }

    #[test]
    fn signal_projection_close_is_handled_only_while_open() {
        let owner = leptos::prelude::Owner::new();
        owner.with(|| {
            let (open, set_open) = signal(true);

            assert!(close_signal_projection(open, set_open));
            assert!(!open.get_untracked());
            assert!(!close_signal_projection(open, set_open));
        });
    }
}
