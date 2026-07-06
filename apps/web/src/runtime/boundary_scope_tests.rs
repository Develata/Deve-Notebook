use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn web_runtime_boundary_scope_helpers_do_not_import_use_core_callbacks() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scoped_dirs = [
        manifest_dir.join("src/editor"),
        manifest_dir.join("src/components"),
        manifest_dir.join("src/hooks"),
    ];
    let excluded_dirs = [manifest_dir.join("src/hooks/use_core")];
    let mut violations = Vec::new();
    for scoped_dir in scoped_dirs {
        collect_rs_files_excluding(&scoped_dir, &excluded_dirs, &mut |path| {
            let content = fs::read_to_string(path).expect("read web source");
            if imports_use_core_callbacks_scope(&content) {
                violations.push(path.strip_prefix(manifest_dir).unwrap().to_path_buf());
            }
        });
    }

    assert!(
        violations.is_empty(),
        "editor/sync/chat scope helpers must import runtime/scope_client directly: {violations:?}"
    );
}

fn collect_rs_files_excluding(
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

fn imports_use_core_callbacks_scope(content: &str) -> bool {
    let compact = content
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect::<String>();
    let direct_path = format!(
        "{}::{}::callbacks_scope",
        hooks_segment(),
        use_core_segment()
    );
    let alias_path = format!("{}::{}::callbacks_scope", hooks_alias(), use_core_segment());
    let generic_alias_path = format!("::{}::callbacks_scope", use_core_segment());
    compact.contains(&direct_path)
        || compact.contains(&alias_path)
        || compact.contains(&generic_alias_path)
        || grouped_import_contains(
            &compact,
            &format!("{}::{}::{{", hooks_segment(), use_core_segment()),
            "callbacks_scope",
        )
        || grouped_import_contains(
            &compact,
            &format!("{}::{}::{{", hooks_alias(), use_core_segment()),
            "callbacks_scope",
        )
        || grouped_import_contains(
            &compact,
            &format!("::{}::{{", use_core_segment()),
            "callbacks_scope",
        )
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

fn hooks_path() -> &'static str {
    concat!("crate", "::", "hooks")
}

fn use_core_segment() -> &'static str {
    concat!("use_", "core")
}

fn hooks_segment() -> &'static str {
    concat!("hoo", "ks")
}

fn hooks_alias() -> &'static str {
    "hooks_alias"
}

#[test]
fn web_runtime_boundary_scope_helper_detects_direct_use_core_callbacks_scope_import() {
    let source = format!(
        "use {}::{}::callbacks_scope::LocalScopeSignals;",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_callbacks_scope(&source));
}

#[test]
fn web_runtime_boundary_scope_helper_detects_grouped_use_core_callbacks_scope_import() {
    let source = format!(
        "use {}::{}::{{ callbacks_scope, EditorContext }};",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_callbacks_scope(&source));
}

#[test]
fn web_runtime_boundary_scope_helper_detects_alias_use_core_callbacks_scope_import() {
    let source = format!(
        "use {} as {}; use {}::{}::callbacks_scope::LocalScopeSignals;",
        hooks_path(),
        hooks_alias(),
        hooks_alias(),
        use_core_segment()
    );
    assert!(imports_use_core_callbacks_scope(&source));
}

#[test]
fn web_runtime_boundary_scope_helper_detects_arbitrary_alias_use_core_callbacks_scope_import() {
    let source = format!(
        "use {} as h; use h::{}::callbacks_scope::LocalScopeSignals;",
        hooks_path(),
        use_core_segment()
    );
    assert!(imports_use_core_callbacks_scope(&source));
}

#[test]
fn web_runtime_boundary_scope_helper_detects_alias_grouped_callbacks_scope_import() {
    let source = format!(
        "use {} as {}; use {}::{}::{{ callbacks_scope, EditorContext }};",
        hooks_path(),
        hooks_alias(),
        hooks_alias(),
        use_core_segment()
    );
    assert!(imports_use_core_callbacks_scope(&source));
}

#[test]
fn web_runtime_boundary_scope_helper_allows_runtime_scope_client_import() {
    let source = "use crate::runtime::scope_client::LocalScopeSignals;";
    assert!(!imports_use_core_callbacks_scope(source));
}
