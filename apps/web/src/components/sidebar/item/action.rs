//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 11_ui_design/index#context-action-surface
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::doc_shell_path::is_doc_shell_path_representable;
use crate::components::dropdown::AnchorRect;
use crate::context_action::{
    ContextActionId, ContextActionIntent, ContextActionReadiness, ContextActionResolveRequest,
    resolve_context_action,
};
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
    readiness: Signal<ContextActionReadiness>,
    delete_req: Callback<String>,
    open_search: Callback<String>,
    copy_absolute_path: Callback<String>,
    reveal_in_system_explorer: Callback<String>,
) -> Callback<ContextActionIntent> {
    Callback::new(move |intent: ContextActionIntent| {
        leptos::logging::log!(
            "item.rs handle_action called: action_id={}",
            intent.action_id.stable_id(),
        );
        let resolve_request = ContextActionResolveRequest::new(intent, readiness.get_untracked());
        let Some(resolved) = resolve_context_action(resolve_request) else {
            return;
        };

        let path = resolved.intent.target.path;
        match resolved.descriptor.id {
            ContextActionId::Rename => {
                if let Some(prefill) = build_rename_prefill(&path) {
                    open_search.run(prefill);
                }
            }
            ContextActionId::Delete => delete_req.run(path),
            ContextActionId::Copy => {
                if let Some(prefill) = build_prefill_command("cp", &path, None) {
                    open_search.run(prefill);
                }
            }
            ContextActionId::OpenInNewWindow => {
                if let Some(window) = web_sys::window()
                    && let Ok(href) = window.location().href()
                {
                    let encoded: String = encode_uri_component(&path).into();
                    let url = build_new_window_url(&href, &encoded);
                    let _ = window.open_with_url_and_target(&url, "_blank");
                }
            }
            ContextActionId::MoveTo => {
                if let Some(prefill) = build_prefill_command("mv", &path, None) {
                    open_search.run(prefill);
                }
            }
            ContextActionId::CopyAbsolutePath => copy_absolute_path.run(path),
            ContextActionId::RevealInSystemExplorer => reveal_in_system_explorer.run(path),
            ContextActionId::ExportPdf => {}
        }
    })
}

fn build_prefill_command(cmd: &str, src: &str, dst_with_cursor: Option<String>) -> Option<String> {
    let src_text = quote_arg(src)?;
    let dst_text = match dst_with_cursor {
        Some(dst) => quote_arg_with_cursor(&dst)?,
        None => "\"|\"".to_string(),
    };
    Some(format!(">{} {} {}", cmd, src_text, dst_text))
}

fn build_rename_prefill(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if !is_doc_shell_path_representable(&normalized) {
        return None;
    }
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
    let (path, query) = base
        .split_once('?')
        .map_or((base, ""), |(path, query)| (path, query));
    let query = replace_doc_query_param(query, doc_param);
    let suffix = if hash.is_empty() {
        String::new()
    } else {
        format!("#{hash}")
    };
    format!("{path}?{query}{suffix}")
}

fn replace_doc_query_param(query: &str, doc_param: &str) -> String {
    let mut pairs = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter(|pair| pair.split_once('=').map_or(*pair, |(key, _)| key) != "doc")
        .map(str::to_string)
        .collect::<Vec<_>>();
    pairs.push(format!("doc={doc_param}"));
    pairs.join("&")
}

fn quote_arg(arg: &str) -> Option<String> {
    is_doc_shell_path_representable(arg).then(|| format!("\"{}\"", arg))
}

fn quote_arg_with_cursor(arg: &str) -> Option<String> {
    if arg.chars().filter(|ch| *ch == '|').count() > 1 {
        return None;
    }
    let without_cursor = arg.replace('|', "");
    is_doc_shell_path_representable(&without_cursor).then(|| format!("\"{}\"", arg))
}

#[cfg(test)]
mod tests;
