//! plan_ref:
//!   - 08_auth#browser-auth-state
//!   - 08_auth#mode-aware-logout-projection
//!
//! Component-scoped browser callback ownership.
//!
//! DOM event listeners and timers must retire with the Leptos owner that created
//! them.  The lifetime token is deliberately presentation-only: it prevents
//! late callbacks from touching disposed signals and owns no session authority.

use leptos::prelude::{GetValue, StoredValue, UpdateValue, WithValue, on_cleanup};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub(crate) struct BrowserRuntimeLifetime {
    active: Arc<AtomicBool>,
    ancestors: Vec<Arc<AtomicBool>>,
}

impl BrowserRuntimeLifetime {
    pub(crate) fn new() -> Self {
        let lifetime = Self::detached();
        let cleanup = lifetime.clone();
        on_cleanup(move || cleanup.shutdown());
        lifetime
    }

    fn detached() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            ancestors: Vec::new(),
        }
    }

    pub(crate) fn child_scope(&self) -> Self {
        let mut ancestors = self.ancestors.clone();
        ancestors.push(self.active.clone());
        let lifetime = Self {
            active: Arc::new(AtomicBool::new(true)),
            ancestors,
        };
        let cleanup = lifetime.clone();
        on_cleanup(move || cleanup.shutdown());
        lifetime
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
            && self
                .ancestors
                .iter()
                .all(|ancestor| ancestor.load(Ordering::Acquire))
    }

    pub(crate) fn shutdown(&self) {
        self.active.store(false, Ordering::Release);
    }

    pub(crate) fn register_event_listener<F>(
        &self,
        target: &web_sys::EventTarget,
        event_name: &'static str,
        mut handler: F,
    ) where
        F: FnMut(web_sys::Event) + 'static,
    {
        use wasm_bindgen::{JsCast, closure::Closure};

        let callback_lifetime = self.clone();
        let callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if callback_lifetime.is_active() {
                handler(event);
            }
        }) as Box<dyn FnMut(_)>);
        if target
            .add_event_listener_with_callback(event_name, callback.as_ref().unchecked_ref())
            .is_err()
        {
            leptos::logging::warn!(
                "deve_browser_runtime category=listener_registration_failed event={event_name}"
            );
            return;
        }

        let cleanup_target = StoredValue::new_local(Some(target.clone()));
        let cleanup_callback = StoredValue::new_local(Some(callback));
        on_cleanup(move || {
            if let Some(target) = cleanup_target.try_get_value().flatten() {
                cleanup_callback.with_value(|callback| {
                    if let Some(callback) = callback {
                        let _ = target.remove_event_listener_with_callback(
                            event_name,
                            callback.as_ref().unchecked_ref(),
                        );
                    }
                });
            }
            cleanup_callback.update_value(|callback| drop(callback.take()));
            cleanup_target.update_value(|target| drop(target.take()));
        });
    }

    #[cfg(not(all(test, not(target_arch = "wasm32"))))]
    pub(crate) fn schedule_timeout<F>(&self, timeout_ms: u32, callback: F)
    where
        F: FnOnce() + 'static,
    {
        let callback_lifetime = self.clone();
        let timeout = gloo_timers::callback::Timeout::new(timeout_ms, move || {
            if callback_lifetime.is_active() {
                callback();
            }
        });
        let timeout = StoredValue::new_local(Some(timeout));
        on_cleanup(move || {
            timeout.update_value(|timeout| {
                if let Some(timeout) = timeout.take() {
                    timeout.cancel();
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::BrowserRuntimeLifetime;
    use futures::channel::oneshot;
    use futures::executor::LocalPool;
    use futures::task::LocalSpawnExt;
    use leptos::prelude::Owner;
    use leptos::prelude::{Set, signal};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn browser_runtime_lifetime_rejects_callbacks_after_shutdown() {
        let lifetime = BrowserRuntimeLifetime::detached();
        assert!(lifetime.is_active());

        lifetime.shutdown();

        assert!(!lifetime.is_active());
    }

    #[test]
    fn browser_runtime_lifetime_retires_when_owner_cleans_up() {
        let owner = Owner::new();
        let lifetime = owner.with(BrowserRuntimeLifetime::new);
        assert!(lifetime.is_active());

        owner.cleanup();

        assert!(!lifetime.is_active());
    }

    #[test]
    fn late_async_completion_is_rejected_before_disposed_signal_access() {
        let owner = Owner::new();
        let mut executor = LocalPool::new();
        let spawner = executor.spawner();
        let (finish, deferred) = oneshot::channel::<()>();
        let completion_applied = Arc::new(AtomicBool::new(false));
        let completion_probe = completion_applied.clone();

        owner.with(|| {
            let (_, set_value) = signal(0u8);
            let lifetime = BrowserRuntimeLifetime::new();
            spawner
                .spawn_local(async move {
                    let _ = deferred.await;
                    if !lifetime.is_active() {
                        return;
                    }
                    set_value.set(1);
                    completion_probe.store(true, Ordering::Release);
                })
                .expect("spawn deferred completion");
        });
        executor.run_until_stalled();

        owner.cleanup();
        finish.send(()).expect("finish deferred completion");
        executor.run_until_stalled();

        assert!(!completion_applied.load(Ordering::Acquire));
    }

    #[test]
    fn child_scope_retires_with_child_owner_while_root_remains_active() {
        let root_owner = Owner::new();
        let root = root_owner.with(BrowserRuntimeLifetime::new);
        let child_owner = Owner::new();
        let child = child_owner.with(|| root.child_scope());

        child_owner.cleanup();

        assert!(root.is_active());
        assert!(!child.is_active());
    }

    #[test]
    fn child_scope_rejects_completion_when_root_retires() {
        let root_owner = Owner::new();
        let root = root_owner.with(BrowserRuntimeLifetime::new);
        let child_owner = Owner::new();
        let child = child_owner.with(|| root.child_scope());

        root_owner.cleanup();

        assert!(!root.is_active());
        assert!(!child.is_active());
    }
}
