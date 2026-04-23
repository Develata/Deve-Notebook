use gloo_timers::callback::Timeout;
use leptos::prelude::*;

pub(crate) fn show_sync_banner(set_sync_banner: WriteSignal<Option<String>>, message: String) {
    set_sync_banner.set(Some(message));
}

pub(crate) fn warn_sync_banner(set_sync_banner: WriteSignal<Option<String>>, message: String) {
    leptos::logging::warn!("{}", message);
    show_sync_banner(set_sync_banner, message);
}

pub(crate) fn show_temporary_sync_banner(
    sync_banner: ReadSignal<Option<String>>,
    set_sync_banner: WriteSignal<Option<String>>,
    message: String,
) {
    set_sync_banner.set(Some(message.clone()));
    Timeout::new(1800, move || {
        if sync_banner.get_untracked().as_deref() == Some(message.as_str()) {
            set_sync_banner.set(None);
        }
    })
    .forget();
}
