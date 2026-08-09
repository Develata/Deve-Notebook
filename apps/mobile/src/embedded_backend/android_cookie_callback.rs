//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Process-local completion registry for Android CookieManager writes. It
//! carries only opaque request ids and typed outcomes; cookie material never
//! enters this registry.

#[cfg(any(target_os = "android", test))]
use std::sync::{Mutex, OnceLock};

use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AndroidCookieCompletion {
    Retained,
    Rejected,
    NotRetained,
    VerificationFailed,
    InvalidCode,
    SetupFailed,
}

impl AndroidCookieCompletion {
    pub(super) fn from_platform_code(code: i32) -> Self {
        match code {
            1 => Self::Retained,
            2 => Self::Rejected,
            3 => Self::NotRetained,
            4 => Self::VerificationFailed,
            5 => Self::SetupFailed,
            _ => Self::InvalidCode,
        }
    }

    pub(super) fn failure_code(self) -> Option<&'static str> {
        match self {
            Self::Retained => None,
            Self::Rejected => Some("android_native_cookie_callback_rejected"),
            Self::NotRetained => Some("android_native_cookie_not_retained"),
            Self::VerificationFailed => Some("android_native_cookie_verification_failed"),
            Self::InvalidCode => Some("android_native_cookie_callback_invalid"),
            Self::SetupFailed => Some("android_native_cookie_jni_setup_failed"),
        }
    }
}

#[derive(Default)]
struct AndroidCookieCallbackRegistry {
    next_request_id: i64,
    pending: Option<(i64, oneshot::Sender<AndroidCookieCompletion>)>,
}

impl AndroidCookieCallbackRegistry {
    fn register(
        &mut self,
    ) -> Result<(i64, oneshot::Receiver<AndroidCookieCompletion>), &'static str> {
        if self.pending.is_some() {
            return Err("android_native_cookie_callback_already_pending");
        }
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or("android_native_cookie_request_id_exhausted")?;
        let request_id = self.next_request_id;
        let (sender, receiver) = oneshot::channel();
        self.pending = Some((request_id, sender));
        Ok((request_id, receiver))
    }

    fn complete(&mut self, request_id: i64, completion: AndroidCookieCompletion) -> bool {
        if !matches!(self.pending.as_ref(), Some((pending_id, _)) if *pending_id == request_id) {
            return false;
        }
        self.pending
            .take()
            .is_some_and(|(_, sender)| sender.send(completion).is_ok())
    }

    fn cancel(&mut self, request_id: i64) -> bool {
        if !matches!(self.pending.as_ref(), Some((pending_id, _)) if *pending_id == request_id) {
            return false;
        }
        self.pending.take().is_some()
    }
}

#[cfg(any(target_os = "android", test))]
fn callback_registry() -> &'static Mutex<AndroidCookieCallbackRegistry> {
    static REGISTRY: OnceLock<Mutex<AndroidCookieCallbackRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(AndroidCookieCallbackRegistry::default()))
}

#[cfg(any(target_os = "android", test))]
pub(super) struct AndroidCookieCallbackRegistration {
    request_id: i64,
    dispatched: bool,
    resolved: bool,
}

#[cfg(any(target_os = "android", test))]
impl AndroidCookieCallbackRegistration {
    pub(super) fn request_id(&self) -> i64 {
        self.request_id
    }

    pub(super) fn mark_dispatched(&mut self) {
        self.dispatched = true;
    }

    pub(super) fn resolve(&mut self) {
        self.resolved = true;
    }

    pub(super) fn cancel_before_dispatch(&mut self) {
        if !self.dispatched && !self.resolved {
            let _ = cancel_android_cookie_callback(self.request_id);
            self.resolved = true;
        }
    }

    pub(super) async fn await_completion(
        &mut self,
        receiver: oneshot::Receiver<AndroidCookieCompletion>,
        limit: Duration,
    ) -> Result<AndroidCookieCompletion, &'static str> {
        match timeout(limit, receiver).await {
            Ok(Ok(completion)) => {
                self.resolve();
                Ok(completion)
            }
            Ok(Err(_)) => {
                self.resolve();
                Err("android_native_cookie_callback_channel_closed")
            }
            Err(_) => Err("android_native_cookie_callback_timeout"),
        }
    }
}

#[cfg(any(target_os = "android", test))]
impl Drop for AndroidCookieCallbackRegistration {
    fn drop(&mut self) {
        if !self.dispatched && !self.resolved {
            let _ = cancel_android_cookie_callback(self.request_id);
        }
    }
}

#[cfg(any(target_os = "android", test))]
pub(super) fn register_android_cookie_callback() -> Result<
    (
        AndroidCookieCallbackRegistration,
        oneshot::Receiver<AndroidCookieCompletion>,
    ),
    &'static str,
> {
    let (request_id, receiver) = callback_registry()
        .lock()
        .map_err(|_| "android_native_cookie_callback_registry_poisoned")?
        .register()?;
    Ok((
        AndroidCookieCallbackRegistration {
            request_id,
            dispatched: false,
            resolved: false,
        },
        receiver,
    ))
}

#[cfg(any(target_os = "android", test))]
pub(super) fn complete_android_cookie_callback(
    request_id: i64,
    completion: AndroidCookieCompletion,
) -> bool {
    callback_registry()
        .lock()
        .map(|mut registry| registry.complete(request_id, completion))
        .unwrap_or(false)
}

#[cfg(any(target_os = "android", test))]
pub(super) fn cancel_android_cookie_callback(request_id: i64) -> bool {
    callback_registry()
        .lock()
        .map(|mut registry| registry.cancel(request_id))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn global_registry_test_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    #[test]
    fn platform_completion_codes_are_fail_closed() {
        assert_eq!(
            AndroidCookieCompletion::from_platform_code(1),
            AndroidCookieCompletion::Retained
        );
        for (code, expected) in [
            (2, "android_native_cookie_callback_rejected"),
            (3, "android_native_cookie_not_retained"),
            (4, "android_native_cookie_verification_failed"),
            (5, "android_native_cookie_jni_setup_failed"),
            (99, "android_native_cookie_callback_invalid"),
        ] {
            assert_eq!(
                AndroidCookieCompletion::from_platform_code(code).failure_code(),
                Some(expected)
            );
        }
    }

    #[tokio::test]
    async fn completion_is_single_consumer_and_duplicate_is_ignored() {
        let mut registry = AndroidCookieCallbackRegistry::default();
        let (request_id, receiver) = registry.register().expect("register");

        assert!(registry.complete(request_id, AndroidCookieCompletion::Retained));
        assert!(!registry.complete(request_id, AndroidCookieCompletion::Rejected));
        assert_eq!(
            receiver.await.expect("completion"),
            AndroidCookieCompletion::Retained
        );
    }

    #[tokio::test]
    async fn cancellation_rejects_late_completion() {
        let mut registry = AndroidCookieCallbackRegistry::default();
        let (request_id, receiver) = registry.register().expect("register");

        assert!(registry.cancel(request_id));
        assert!(!registry.complete(request_id, AndroidCookieCompletion::Retained));
        assert!(receiver.await.is_err());
    }

    #[tokio::test]
    async fn concurrent_registration_fails_closed_without_replacing_first_waiter() {
        let mut registry = AndroidCookieCallbackRegistry::default();
        let (request_id, receiver) = registry.register().expect("first register");

        assert_eq!(
            registry
                .register()
                .expect_err("concurrent register rejected"),
            "android_native_cookie_callback_already_pending"
        );
        assert!(registry.complete(request_id, AndroidCookieCompletion::Retained));
        assert_eq!(
            receiver.await.expect("first completion"),
            AndroidCookieCompletion::Retained
        );
    }

    #[tokio::test]
    async fn dropped_undispatched_registration_cancels_waiter() {
        let _guard = global_registry_test_lock().lock().await;
        let (mut registration, receiver) = register_android_cookie_callback().expect("register");
        let request_id = registration.request_id();

        registration.cancel_before_dispatch();
        drop(registration);
        assert!(receiver.await.is_err());
        assert!(!complete_android_cookie_callback(
            request_id,
            AndroidCookieCompletion::Retained
        ));

        let (next_registration, next_receiver) =
            register_android_cookie_callback().expect("register after cancellation");
        let next_request_id = next_registration.request_id();
        assert!(next_request_id > request_id);
        drop(next_registration);
        assert!(next_receiver.await.is_err());
    }

    #[tokio::test]
    async fn dropped_dispatched_registration_preserves_tombstone_until_late_completion() {
        let _guard = global_registry_test_lock().lock().await;
        let (mut registration, receiver) = register_android_cookie_callback().expect("register");
        let request_id = registration.request_id();
        registration.mark_dispatched();

        drop(receiver);
        drop(registration);
        assert!(matches!(
            register_android_cookie_callback(),
            Err("android_native_cookie_callback_already_pending")
        ));
        assert!(!complete_android_cookie_callback(
            request_id,
            AndroidCookieCompletion::Retained
        ));

        let (next_registration, next_receiver) =
            register_android_cookie_callback().expect("register after late completion");
        assert!(next_registration.request_id() > request_id);
        drop(next_registration);
        assert!(next_receiver.await.is_err());
    }

    #[tokio::test]
    async fn android_native_session_cookie_callback_failure_timeout_and_late_completion_fail_closed()
     {
        let _guard = global_registry_test_lock().lock().await;

        let (mut retained, retained_receiver) =
            register_android_cookie_callback().expect("retained register");
        let retained_id = retained.request_id();
        retained.mark_dispatched();
        assert!(complete_android_cookie_callback(
            retained_id,
            AndroidCookieCompletion::Retained
        ));
        assert_eq!(
            retained
                .await_completion(retained_receiver, Duration::from_millis(10))
                .await
                .expect("retained completion"),
            AndroidCookieCompletion::Retained
        );

        let (mut closed, closed_receiver) =
            register_android_cookie_callback().expect("closed register");
        let closed_id = closed.request_id();
        closed.mark_dispatched();
        assert!(cancel_android_cookie_callback(closed_id));
        assert_eq!(
            closed
                .await_completion(closed_receiver, Duration::from_millis(10))
                .await,
            Err("android_native_cookie_callback_channel_closed")
        );

        let (mut timed_out, timeout_receiver) =
            register_android_cookie_callback().expect("timeout register");
        let timed_out_id = timed_out.request_id();
        timed_out.mark_dispatched();
        assert_eq!(
            timed_out
                .await_completion(timeout_receiver, Duration::from_millis(1))
                .await,
            Err("android_native_cookie_callback_timeout")
        );
        drop(timed_out);

        assert!(matches!(
            register_android_cookie_callback(),
            Err("android_native_cookie_callback_already_pending")
        ));
        assert!(!complete_android_cookie_callback(
            timed_out_id,
            AndroidCookieCompletion::Retained
        ));

        let (next_registration, next_receiver) =
            register_android_cookie_callback().expect("register after late completion");
        assert!(next_registration.request_id() > timed_out_id);
        drop(next_registration);
        assert!(next_receiver.await.is_err());
    }
}
