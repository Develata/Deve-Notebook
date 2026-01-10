use leptos::prelude::*;
use web_sys::MouseEvent;

pub fn use_layout() -> (
    ReadSignal<i32>,
    Callback<MouseEvent>, // start_resize
    Callback<()>,         // stop_resize
    Callback<MouseEvent>, // do_resize
    ReadSignal<bool>      // is_resizing
) {
    let (sidebar_width, set_sidebar_width) = signal(250);
    let (is_resizing, set_is_resizing) = signal(false);
    
    let start_resize = Callback::new(move |ev: MouseEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
    });
    
    let stop_resize = Callback::new(move |_| {
        set_is_resizing.set(false);
    });
    
    let do_resize = Callback::new(move |ev: MouseEvent| {
        if is_resizing.get_untracked() {
            let new_width = ev.client_x();
            // Clamp width (min 150, max 600)
            if new_width > 150 && new_width < 600 {
                set_sidebar_width.set(new_width);
            }
        }
    });

    (sidebar_width, start_resize, stop_resize, do_resize, is_resizing)
}
