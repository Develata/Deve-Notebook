//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! Web client runtime bands.
//!
//! Infra-first runtime convergence (Phase B+) per
//! `docs/tasks/19_repo_refactor_blueprint.md` §3.3 and
//! `docs/report/runtime-convergence-audit-2026-05-28.md`: scattered runtime
//! logic under `hooks/use_core/` (the `effects_*` / `callbacks_*` prefix
//! families) is migrated here into `runtime/*_client` bands with typed APIs.
//! These modules are Flow Coordination / Object Plane adapters only; they never
//! own ledger, projection, or source-control authority.

pub mod document;
pub mod document_client;
pub mod domain;
pub mod external_changes_client;
pub mod rendering_client;
pub mod scope_client;
pub mod session_client;
pub mod source_control_client;

#[derive(Clone)]
pub struct CoreRuntimeClients {
    pub session: session_client::SessionClient,
    pub scope: scope_client::ScopeClient,
    pub document: document_client::DocumentClient,
    pub source_control: source_control_client::SourceControlClient,
    pub external_changes: external_changes_client::ExternalChangesClient,
    pub rendering: rendering_client::RenderingClient,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn web_runtime_boundary_does_not_import_use_core_internals() {
        let runtime_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
        let mut violations = Vec::new();
        collect_rs_files(&runtime_dir, &mut |path| {
            let content = fs::read_to_string(path).expect("read runtime source");
            if imports_use_core_internals(&content) {
                violations.push(path.strip_prefix(&runtime_dir).unwrap().to_path_buf());
            }
        });

        assert!(
            violations.is_empty(),
            "apps/web/src/runtime must not import hooks/use_core internals: {violations:?}"
        );
    }

    fn collect_rs_files(root: &Path, visit: &mut impl FnMut(&Path)) {
        for entry in fs::read_dir(root).expect("read runtime dir") {
            let path = entry.expect("runtime dir entry").path();
            if path.is_dir() {
                collect_rs_files(&path, visit);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                visit(&path);
            }
        }
    }

    fn imports_use_core_internals(content: &str) -> bool {
        let compact = content
            .chars()
            .filter(|value| !value.is_whitespace())
            .collect::<String>();
        let hooks_path = hooks_path();
        let use_core = use_core_segment();
        if compact.contains(&format!("{hooks_path}::{use_core}")) {
            return true;
        }
        grouped_import_contains(&compact, &format!("{hooks_path}::{{"), use_core)
            || root_grouped_import_contains(&compact, use_core)
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

    fn root_grouped_import_contains(content: &str, use_core: &str) -> bool {
        let root_prefix = format!("{}::{{", crate_root());
        let mut offset = 0;
        while let Some(index) = content[offset..].find(&root_prefix) {
            let body_start = offset + index + root_prefix.len();
            if let Some(body_end) = grouped_import_end(&content[body_start..]) {
                let body = &content[body_start..body_start + body_end];
                if body.contains(&format!("hooks::{use_core}"))
                    || grouped_import_contains(body, "hooks::{", use_core)
                {
                    return true;
                }
                offset = body_start + body_end + 1;
            } else {
                return content[body_start..].contains(&format!("hooks::{use_core}"))
                    || grouped_import_contains(&content[body_start..], "hooks::{", use_core);
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

    #[test]
    fn web_runtime_boundary_detects_direct_use_core_import() {
        let source = format!("use {}::{}::LoadPhase;", hooks_path(), use_core_segment());
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
            "use {}::{{ hooks::{}::LoadPhase }};",
            crate_root(),
            use_core_segment()
        );
        assert!(imports_use_core_internals(&source));
    }

    #[test]
    fn web_runtime_boundary_detects_nested_root_grouped_use_core_import() {
        let source = format!(
            "use {}::{{ runtime, hooks::{{{}::LoadPhase}} }};",
            crate_root(),
            use_core_segment()
        );
        assert!(imports_use_core_internals(&source));
    }

    #[test]
    fn web_runtime_boundary_allows_runtime_domain_import() {
        let source = "use crate::runtime::domain::LoadPhase;";
        assert!(!imports_use_core_internals(source));
    }
}
