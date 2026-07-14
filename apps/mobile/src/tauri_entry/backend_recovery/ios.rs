//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! iOS platform-owned backend recovery control.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSString;
use objc2_ui_kit::{UIAction, UIButton, UIButtonType, UIViewAutoresizing, UIViewController};
use tauri::{WebviewWindow, Wry};

use super::invoke_registered_recovery;

const RECOVERY_CONTROL_TAG: isize = 0x4445_5645;
const PLATFORM_CONTROL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

pub(super) async fn install(window: &WebviewWindow<Wry>) -> Result<(), String> {
    with_root_view(window, |view, mtm| {
        if unsafe { view.viewWithTag(RECOVERY_CONTROL_TAG) }.is_some() {
            return Ok(());
        }
        let title = NSString::from_str("Use Local Backend");
        let handler: RcBlock<dyn Fn(NonNull<UIAction>)> = RcBlock::new(|_| {
            let _ = invoke_registered_recovery();
        });
        let action = unsafe {
            UIAction::actionWithTitle_image_identifier_handler(
                &title,
                None,
                None,
                RcBlock::as_ptr(&handler),
                mtm,
            )
        };
        let button =
            UIButton::buttonWithType_primaryAction(UIButtonType::System, Some(&action), mtm);
        button.setTag(RECOVERY_CONTROL_TAG);
        let bounds = view.bounds();
        let safe_area = view.safeAreaInsets();
        let width = 168.0;
        let height = 44.0;
        button.setFrame(CGRect::new(
            CGPoint::new(
                (bounds.size.width - width - 12.0).max(12.0),
                safe_area.top + 8.0,
            ),
            CGSize::new(width, height),
        ));
        button.setAutoresizingMask(
            UIViewAutoresizing::FlexibleLeftMargin | UIViewAutoresizing::FlexibleBottomMargin,
        );
        view.addSubview(&button);
        Ok(())
    })
    .await
}

pub(super) async fn reset(window: &WebviewWindow<Wry>) -> Result<(), String> {
    remove(window).await?;
    install(window).await
}

pub(super) async fn remove(window: &WebviewWindow<Wry>) -> Result<(), String> {
    with_root_view(window, |view, _mtm| {
        if let Some(control) = unsafe { view.viewWithTag(RECOVERY_CONTROL_TAG) } {
            control.removeFromSuperview();
        }
        Ok(())
    })
    .await
}

async fn with_root_view(
    window: &WebviewWindow<Wry>,
    action: impl FnOnce(&objc2_ui_kit::UIView, MainThreadMarker) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |platform| {
            let view_controller = platform.view_controller();
            if view_controller.is_null() {
                let _ = sender.send(Err(
                    "iOS native recovery view controller unavailable".to_string()
                ));
                return;
            }
            let Some(mtm) = MainThreadMarker::new() else {
                let _ = sender.send(Err(
                    "iOS native recovery callback is not on the main thread".to_string(),
                ));
                return;
            };
            let view_controller = unsafe { &*view_controller.cast::<UIViewController>() };
            view_controller.loadViewIfNeeded();
            let Some(view) = view_controller.view() else {
                let _ = sender.send(Err("iOS native recovery root view unavailable".to_string()));
                return;
            };
            let _ = sender.send(action(&view, mtm));
        })
        .map_err(|error| error.to_string())?;
    tokio::time::timeout(PLATFORM_CONTROL_TIMEOUT, receiver)
        .await
        .map_err(|_| "iOS native recovery control operation timed out".to_string())?
        .map_err(|_| "iOS native recovery control result channel closed".to_string())?
}
