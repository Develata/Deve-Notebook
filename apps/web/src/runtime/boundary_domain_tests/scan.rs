//! plan_ref:
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Import scanners for Web thin-client boundary checks.

use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn collect_rs_files(root: &Path, visit: &mut impl FnMut(&Path)) {
    collect_rs_files_excluding(root, &[], visit);
}

pub(super) fn collect_rs_files_excluding(
    root: &Path,
    excluded_dirs: &[PathBuf],
    visit: &mut impl FnMut(&Path),
) {
    for entry in fs::read_dir(root).expect("read runtime dir") {
        let path = entry.expect("runtime dir entry").path();
        if excluded_dirs
            .iter()
            .any(|excluded| path == excluded.as_path() || path.starts_with(excluded))
        {
            continue;
        }
        if path.is_dir() {
            collect_rs_files_excluding(&path, excluded_dirs, visit);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            visit(&path);
        }
    }
}

pub(super) fn imports_use_core_internals(content: &str) -> bool {
    let compact = content
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect::<String>();
    let use_core = use_core_segment();
    if imports_hooks_alias(&compact) {
        return true;
    }
    contains_hooks_use_core_path(&compact, use_core)
}

pub(super) fn imports_use_core_domain_type_reexports(content: &str) -> bool {
    let compact = content
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect::<String>();
    web_domain_type_segments()
        .iter()
        .any(|type_name| imports_use_core_domain_type(&compact, type_name))
}

fn imports_use_core_domain_type(content: &str, type_name: &str) -> bool {
    let use_core = use_core_segment();
    content.contains(&format!("{}::{use_core}::{type_name}", hooks_segment()))
        || content.contains(&format!("{}::{use_core}::{type_name}", hooks_alias()))
        || content.contains(&format!(
            "{}::{use_core}::types::{type_name}",
            hooks_segment()
        ))
        || content.contains(&format!(
            "{}::{use_core}::types::{type_name}",
            hooks_alias()
        ))
        || grouped_import_contains(
            content,
            &format!("{}::{use_core}::{{", hooks_segment()),
            type_name,
        )
        || grouped_import_contains(
            content,
            &format!("{}::{use_core}::{{", hooks_alias()),
            type_name,
        )
        || grouped_import_contains(
            content,
            &format!("{}::{use_core}::types::{{", hooks_segment()),
            type_name,
        )
        || grouped_import_contains(
            content,
            &format!("{}::{use_core}::types::{{", hooks_alias()),
            type_name,
        )
        || grouped_import_contains(content, &hooks_group_prefix(), use_core)
            && grouped_import_contains(content, &hooks_group_prefix(), type_name)
        || grouped_import_contains(content, &hooks_alias_group_prefix(), use_core)
            && grouped_import_contains(content, &hooks_alias_group_prefix(), type_name)
}

fn web_domain_type_segments() -> &'static [&'static str] {
    &[
        "AiBackendMode",
        "ChatMessage",
        "LoadPhase",
        "PeerSession",
        "PendingBranchSwitch",
        "PendingBranchTarget",
        "PendingOpsPreview",
        "PendingRepoSwitch",
        "RepoRemoveRequest",
        "RepoRenameRequest",
        "RepoSwitchRequest",
        "SearchHit",
        "SyncModeState",
    ]
}

fn imports_hooks_alias(content: &str) -> bool {
    let alias_marker = hooks_alias_marker();
    content.contains(&format!("::{alias_marker}"))
        || content.contains(&format!("{{{alias_marker}"))
        || content.contains(&format!(",{alias_marker}"))
        || grouped_import_contains(content, &hooks_group_prefix(), "selfas")
}

fn contains_hooks_use_core_path(content: &str, use_core: &str) -> bool {
    content.contains(&hooks_use_core_path(use_core))
        || grouped_import_contains(content, &hooks_group_prefix(), use_core)
}

fn grouped_import_contains(content: &str, prefix: &str, needle: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = content[offset..].find(prefix) {
        let body_start = offset + index + prefix.len();
        if let Some(body_end) = grouped_import_end(&content[body_start..]) {
            if content[body_start..body_start + body_end].contains(needle) {
                return true;
            }
            offset = body_start + body_end + 1;
        } else {
            return content[body_start..].contains(needle);
        }
    }
    false
}

fn grouped_import_end(content: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (index, value) in content.char_indices() {
        match value {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn crate_root() -> &'static str {
    "crate"
}

fn hooks_path() -> &'static str {
    concat!("crate", "::", "hooks")
}

fn use_core_segment() -> &'static str {
    concat!("use_", "core")
}

fn hooks_segment() -> &'static str {
    concat!("hoo", "ks")
}

fn hooks_alias_marker() -> String {
    format!("{}as", hooks_segment())
}

fn hooks_alias() -> &'static str {
    "hooks_alias"
}

fn hooks_group_prefix() -> String {
    format!("{}::{{", hooks_segment())
}

fn hooks_alias_group_prefix() -> String {
    format!("{}::{{", hooks_alias())
}

fn hooks_use_core_path(use_core: &str) -> String {
    format!("{}::{use_core}", hooks_segment())
}

fn relative_hooks_path(depth: usize) -> String {
    format!("{}{}", "super::".repeat(depth), hooks_segment())
}

fn relative_module_path(depth: usize) -> String {
    "super::".repeat(depth).trim_end_matches("::").to_string()
}

#[test]
fn web_runtime_boundary_detects_direct_use_core_import() {
    let source = format!("use {}::{}::LoadPhase;", hooks_path(), use_core_segment());
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_hooks_alias_import() {
    let source = format!(
        "use {} as hooks_alias; use hooks_alias::{}::LoadPhase;",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_relative_use_core_import() {
    let source = format!(
        "use {}::{}::LoadPhase;",
        relative_hooks_path(1),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_deep_relative_hooks_alias_import() {
    let source = format!(
        "use {} as hooks_alias; use hooks_alias::{}::LoadPhase;",
        relative_hooks_path(2),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_grouped_use_core_import() {
    let source = format!(
        "use {}::{{ runtime, {}::LoadPhase }};",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_root_grouped_use_core_import() {
    let source = format!(
        "use {}::{{ {}::{}::LoadPhase }};",
        crate_root(),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_root_grouped_hooks_alias_import() {
    let source = format!(
        "use {}::{{ {} as hooks_alias }}; use hooks_alias::{}::LoadPhase;",
        crate_root(),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_nested_hooks_self_alias_import() {
    let source = format!(
        "use {}::{{ {}::{{ self as hooks_alias }} }}; use hooks_alias::{}::LoadPhase;",
        crate_root(),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_nested_root_grouped_use_core_import() {
    let source = format!(
        "use {}::{{ runtime, {}::{{{}::LoadPhase}} }};",
        crate_root(),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_relative_fully_qualified_use_core_path() {
    let source = format!(
        "type State = {}::{}::LoadPhase;",
        relative_hooks_path(3),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_crate_alias_use_core_path() {
    let source = format!(
        "use {} as app; type State = self::app::{}::{}::LoadPhase;",
        crate_root(),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_relative_grouped_use_core_import() {
    let source = format!(
        "use {}::{{ {}::{}::LoadPhase }};",
        relative_module_path(1),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_relative_nested_grouped_use_core_import() {
    let source = format!(
        "use {}::{{ {}::{{ self, {}::LoadPhase }} }};",
        relative_module_path(1),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_relative_grouped_hooks_alias_import() {
    let source = format!(
        "use {}::{{ {} as hooks_alias }}; use hooks_alias::{}::LoadPhase;",
        relative_module_path(2),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_detects_relative_nested_hooks_self_alias_import() {
    let source = format!(
        "use {}::{{ {}::{{ self as hooks_alias }} }}; use hooks_alias::{}::LoadPhase;",
        relative_module_path(1),
        hooks_segment(),
        use_core_segment()
    );
    assert!(imports_use_core_internals(&source));
}

#[test]
fn web_runtime_boundary_allows_runtime_domain_import() {
    let source = "use crate::runtime::domain::LoadPhase;";
    assert!(!imports_use_core_internals(source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_direct_use_core_domain_type_import() {
    let source = format!("use {}::{}::LoadPhase;", hooks_path(), use_core_segment());
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_use_core_types_domain_type_import() {
    let source = format!(
        "use {}::{}::types::ChatMessage;",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_grouped_use_core_types_domain_type_import() {
    let source = format!(
        "use {}::{}::types::{{ ChatMessage, LoadPhase }};",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_grouped_use_core_domain_type_import() {
    let source = format!(
        "use {}::{}::{{ EditorContext, PendingRepoSwitch }};",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_hooks_alias_domain_type_import() {
    let source = format!(
        "use {} as {}; use {}::{}::LoadPhase;",
        hooks_path(),
        hooks_alias(),
        hooks_alias(),
        use_core_segment()
    );
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_hooks_alias_grouped_domain_type_import() {
    let source = format!(
        "use {} as {}; use {}::{{ {}::{{ EditorContext, PendingRepoSwitch }} }};",
        hooks_path(),
        hooks_alias(),
        hooks_alias(),
        use_core_segment()
    );
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_detects_repo_request_reexports() {
    let source = format!(
        "use {}::{}::{{ RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest }};",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_allows_use_core_composition_types() {
    let source = format!(
        "use {}::{}::EditorContext;",
        hooks_path(),
        use_core_segment()
    );
    assert!(!imports_use_core_domain_type_reexports(&source));
}

#[test]
fn web_runtime_boundary_editor_domain_allows_runtime_domain_import() {
    let source = "use crate::runtime::domain::PendingRepoSwitch;";
    assert!(!imports_use_core_domain_type_reexports(source));
}
