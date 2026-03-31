use crate::components::dropdown::AnchorRect;
use crate::components::sidebar_menu::MenuAction;
use leptos::prelude::*;
use leptos::reactive::traits::GetUntracked;
use wasm_bindgen::JsCast;

pub(super) fn create_menu_anchor(target: Option<web_sys::EventTarget>) -> AnchorRect {
    if let Some(target) = target
        && let Ok(el) = target.dyn_into::<web_sys::Element>()
    {
        let rect = el.get_bounding_client_rect();
        return AnchorRect {
            top: rect.top(),
            bottom: rect.bottom(),
            left: rect.left(),
            right: rect.right(),
        };
    }

    AnchorRect {
        top: 0.0,
        bottom: 0.0,
        left: 0.0,
        right: 0.0,
    }
}

pub(super) fn create_action_handler(
    is_readonly: Signal<bool>,
    delete_req: Callback<String>,
    open_search: Callback<String>,
    path: String,
) -> Callback<MenuAction> {
    Callback::new(move |action: MenuAction| {
        leptos::logging::log!("item.rs handle_action called: action={:?}", action);
        if is_readonly.get_untracked() && !matches!(action, MenuAction::OpenInNewWindow) {
            return;
        }

        match action {
            MenuAction::Rename => {
                open_search.run(build_prefill_command("mv", &path, None));
            }
            MenuAction::Delete => delete_req.run(path.clone()),
            MenuAction::Copy => {
                open_search.run(build_prefill_command("cp", &path, None));
            }
            MenuAction::OpenInNewWindow => {
                if let Some(window) = web_sys::window()
                    && let Ok(href) = window.location().href()
                {
                    let url = format!("{}?doc={}", href, path);
                    let _ = window.open_with_url_and_target(&url, "_blank");
                }
            }
            MenuAction::MoveTo => {
                open_search.run(build_prefill_command("mv", &path, None));
            }
        }
    })
}

fn build_prefill_command(cmd: &str, src: &str, dst_with_cursor: Option<String>) -> String {
    let src_text = quote_arg(src);
    let dst_text = match dst_with_cursor {
        Some(dst) => format!("\"{}\"", sanitize_arg(&dst)),
        None => "\"|\"".to_string(),
    };
    format!(">{} {} {}", cmd, src_text, dst_text)
}

fn quote_arg(arg: &str) -> String {
    format!("\"{}\"", sanitize_arg(arg))
}

fn sanitize_arg(arg: &str) -> String {
    arg.replace('"', "'")
}
