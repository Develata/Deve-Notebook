//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::components::dropdown::AnchorRect;
use crate::components::sidebar_menu::MenuAction;
use js_sys::encode_uri_component;
use leptos::prelude::*;
use leptos::reactive::traits::GetUntracked;
use std::path::Path;
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
                open_search.run(build_rename_prefill(&path));
            }
            MenuAction::Delete => delete_req.run(path.clone()),
            MenuAction::Copy => {
                open_search.run(build_prefill_command("cp", &path, None));
            }
            MenuAction::OpenInNewWindow => {
                if let Some(window) = web_sys::window()
                    && let Ok(href) = window.location().href()
                {
                    let encoded: String = encode_uri_component(&path).into();
                    let url = build_new_window_url(&href, &encoded);
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

fn build_rename_prefill(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let path_ref = Path::new(&normalized);
    let parent = path_ref
        .parent()
        .and_then(|p| p.to_str())
        .filter(|p| !p.is_empty())
        .map(|p| p.replace('\\', "/"));
    let name = path_ref
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(normalized.as_str());
    let renamed = match path_ref.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            let stem = path_ref
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or(name);
            format!("{stem}|.{ext}")
        }
        None => format!("{name}|"),
    };
    let dst = match parent {
        Some(parent) => format!("{parent}/{renamed}"),
        None => renamed,
    };
    build_prefill_command("mv", &normalized, Some(dst))
}

fn build_new_window_url(href: &str, doc_param: &str) -> String {
    let (base, hash) = href
        .split_once('#')
        .map_or((href, ""), |(base, hash)| (base, hash));
    let sep = if base.contains('?') { '&' } else { '?' };
    let suffix = if hash.is_empty() {
        String::new()
    } else {
        format!("#{hash}")
    };
    format!("{base}{sep}doc={doc_param}{suffix}")
}

fn quote_arg(arg: &str) -> String {
    format!("\"{}\"", sanitize_arg(arg))
}

fn sanitize_arg(arg: &str) -> String {
    arg.replace('"', "'")
}

#[cfg(test)]
mod tests;
