//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Web runtime import boundary tests.

#[cfg(test)]
mod scan;

#[cfg(test)]
mod tests {
    use super::scan::{
        collect_rs_files, collect_rs_files_excluding, imports_use_core_domain_type_reexports,
        imports_use_core_internals,
    };
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

    #[test]
    fn web_runtime_boundary_consumers_domain_types_do_not_import_use_core_reexports() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let consumer_roots = [
            manifest_dir.join("src/editor"),
            manifest_dir.join("src/components"),
            manifest_dir.join("src/hooks"),
        ];
        let excluded_dirs = [manifest_dir.join("src/hooks/use_core")];
        let mut violations = Vec::new();
        for consumer_root in consumer_roots {
            collect_rs_files_excluding(&consumer_root, &excluded_dirs, &mut |path| {
                let content = fs::read_to_string(path).expect("read web consumer source");
                if imports_use_core_domain_type_reexports(&content) {
                    violations.push(path.strip_prefix(manifest_dir).unwrap().to_path_buf());
                }
            });
        }

        assert!(
            violations.is_empty(),
            "web runtime consumers must import runtime/domain shared types directly: {violations:?}"
        );
    }
}
