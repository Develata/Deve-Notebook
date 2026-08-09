//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Android CookieManager handoff. The platform completion callback is the
//! ordering boundary; an immediate read after `setCookie` is not accepted as
//! proof that a replacement cookie has been installed.

use jni::objects::{JObject, JValue};

use super::android_cookie_callback::{
    AndroidCookieCompletion, complete_android_cookie_callback, register_android_cookie_callback,
};
use super::cookie::MobileNativeSessionCookie;

const ANDROID_COOKIE_CALLBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub(super) async fn install_native_session_cookie_confirmed<R: tauri::Runtime>(
    webview: &tauri::WebviewWindow<R>,
    cookie: &MobileNativeSessionCookie,
) -> Result<(), String> {
    let install_url = cookie.android_install_url();
    let verification_url = cookie.android_verification_url();
    let set_cookie = cookie.android_set_cookie_value();
    let (mut registration, receiver) =
        register_android_cookie_callback().map_err(str::to_string)?;
    let request_id = registration.request_id();

    let dispatch = webview.with_webview(move |platform| {
        platform
            .jni_handle()
            .exec(move |env, activity, android_webview| {
                let completion = begin_android_cookie_install(
                    env,
                    activity,
                    android_webview,
                    request_id,
                    &install_url,
                    &verification_url,
                    &set_cookie,
                );
                if let Some(completion) = completion {
                    let _ = complete_android_cookie_callback(request_id, completion);
                }
            });
    });
    if dispatch.is_err() {
        registration.cancel_before_dispatch();
        return Err("android_native_cookie_webview_dispatch_failed".to_string());
    }
    registration.mark_dispatched();

    let completion = registration
        .await_completion(receiver, ANDROID_COOKIE_CALLBACK_TIMEOUT)
        .await
        .map_err(str::to_string)?;
    match completion.failure_code() {
        None => Ok(()),
        Some(code) => Err(code.to_string()),
    }
}

fn begin_android_cookie_install(
    env: &mut jni::JNIEnv<'_>,
    activity: &JObject<'_>,
    android_webview: &JObject<'_>,
    request_id: i64,
    install_url: &str,
    verification_url: &str,
    set_cookie: &str,
) -> Option<AndroidCookieCompletion> {
    let result = (|| -> Result<bool, jni::errors::Error> {
        let install_url = JObject::from(env.new_string(install_url)?);
        let verification_url = JObject::from(env.new_string(verification_url)?);
        let set_cookie = JObject::from(env.new_string(set_cookie)?);
        env.call_method(
            activity,
            "installNativeSessionCookie",
            "(JLandroid/webkit/WebView;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Z",
            &[
                JValue::Long(request_id),
                JValue::Object(android_webview),
                JValue::Object(&install_url),
                JValue::Object(&verification_url),
                JValue::Object(&set_cookie),
            ],
        )?
        .z()
    })();
    match result {
        Ok(true) => None,
        Ok(false) => Some(AndroidCookieCompletion::SetupFailed),
        Err(jni::errors::Error::JavaException) => {
            let _ = env.exception_clear();
            Some(AndroidCookieCompletion::SetupFailed)
        }
        Err(_) => Some(AndroidCookieCompletion::SetupFailed),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_deve_notebook_mobile_MainActivity_nativeSessionCookieInstallCompleted(
    _env: jni::JNIEnv<'_>,
    _activity: JObject<'_>,
    request_id: jni::sys::jlong,
    completion_code: jni::sys::jint,
) {
    let completion = AndroidCookieCompletion::from_platform_code(completion_code);
    let _ = complete_android_cookie_callback(request_id, completion);
}
