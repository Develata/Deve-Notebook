use leptos::prelude::*;
use web_sys::KeyboardEvent;
use crate::i18n::Locale;

pub fn use_shortcuts(
    locale: RwSignal<Locale>,
    show_cmd: ReadSignal<bool>,
    set_show_cmd: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>
) -> impl Fn(KeyboardEvent) + Clone + 'static {
    move |ev: KeyboardEvent| {
        let is_ctrl = ev.meta_key() || ev.ctrl_key();
        let key = ev.key().to_lowercase();
        
        // Ctrl+K: Toggle Command Palette
        if is_ctrl && key == "k" {
            ev.prevent_default();
            ev.stop_propagation(); 
            set_show_cmd.update(|s| *s = !*s);
        }
        
        // Ctrl+L: Toggle Language
        if is_ctrl && key == "l" {
             ev.prevent_default();
             ev.stop_propagation();
             locale.update(|l| *l = l.toggle());
        }

        // Ctrl+O: Open Document Modal
        if is_ctrl && key == "o" {
             ev.prevent_default();
             ev.stop_propagation();
             set_show_open_modal.set(true);
        }
        
        // Escape: Close Command Palette
        if show_cmd.get_untracked() && ev.key() == "Escape" {
             set_show_cmd.set(false);
        }
    }
}
