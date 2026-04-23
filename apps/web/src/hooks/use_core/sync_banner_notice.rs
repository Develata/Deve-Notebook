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

#[cfg(test)]
mod tests {
    use super::{show_sync_banner, warn_sync_banner};
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn show_sync_banner_sets_visible_message() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);

        show_sync_banner(set_sync_banner, "Blocked: repo is switching".to_string());

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("Blocked: repo is switching")
        );
    }

    #[test]
    fn warn_sync_banner_sets_visible_message() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);

        warn_sync_banner(set_sync_banner, "Blocked: write gate closed".to_string());

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("Blocked: write gate closed")
        );
    }
}
