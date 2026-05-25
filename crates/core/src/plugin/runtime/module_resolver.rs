//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use rhai::module_resolvers::FileModuleResolver;
use rhai::{Engine, Module, ModuleResolver, Position, Shared};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

/// File-backed Rhai module resolver constrained to one plugin directory.
#[derive(Debug)]
pub(super) struct GuardedFileModuleResolver {
    inner: FileModuleResolver,
    base_dir: PathBuf,
}

impl GuardedFileModuleResolver {
    pub(super) fn new(base_dir: PathBuf) -> Self {
        Self {
            inner: FileModuleResolver::new_with_path(&base_dir),
            base_dir,
        }
    }

    fn validate_module_path(&self, path: &str) -> Result<(), String> {
        if path.is_empty() {
            return Err("Invalid plugin module import: path must not be empty".to_string());
        }
        if path.contains('\\') {
            return Err(format!(
                "Invalid plugin module import '{}': use forward-slash relative paths",
                path
            ));
        }

        let module_path = Path::new(path);
        if module_path.is_absolute() {
            return Err(format!(
                "Invalid plugin module import '{}': absolute paths are not allowed",
                path
            ));
        }
        for component in module_path.components() {
            if !matches!(component, Component::Normal(_)) {
                return Err(format!(
                    "Invalid plugin module import '{}': only normal relative path segments are allowed",
                    path
                ));
            }
        }

        if let Some(extension) = module_path.extension().and_then(|ext| ext.to_str())
            && extension != "rhai"
        {
            return Err(format!(
                "Invalid plugin module import '{}': module must be a .rhai script",
                path
            ));
        }
        Ok(())
    }

    fn guard_resolved_target(&self, path: &str) -> Result<(), String> {
        self.validate_module_path(path)?;

        let canonical_root = std::fs::canonicalize(&self.base_dir).map_err(|err| {
            format!(
                "Failed to canonicalize plugin module directory {:?}: {}",
                self.base_dir, err
            )
        })?;
        let target = self.inner.get_file_path(path, None);
        let canonical_target = canonicalize_existing_ancestor(&target)?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(format!(
                "Invalid plugin module import '{}': resolved module must stay inside plugin directory",
                path
            ));
        }
        Ok(())
    }
}

impl ModuleResolver for GuardedFileModuleResolver {
    fn resolve(
        &self,
        engine: &Engine,
        source: Option<&str>,
        path: &str,
        pos: Position,
    ) -> Result<Shared<Module>, Box<rhai::EvalAltResult>> {
        self.guard_resolved_target(path)
            .map_err(|err| -> Box<rhai::EvalAltResult> { err.into() })?;
        self.inner.resolve(engine, source, path, pos)
    }
}

fn canonicalize_existing_ancestor(path: &Path) -> Result<PathBuf, String> {
    let mut cursor = path;
    let mut missing = Vec::new();

    loop {
        match std::fs::symlink_metadata(cursor) {
            Ok(_) => {
                let mut canonical = std::fs::canonicalize(cursor).map_err(|err| {
                    format!(
                        "Failed to canonicalize plugin module target {:?}: {}",
                        cursor, err
                    )
                })?;
                for component in missing.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "Failed to stat plugin module target {:?}: {}",
                    cursor, err
                ));
            }
        }

        let Some(file_name) = cursor.file_name() else {
            return Err(format!(
                "Plugin module target {:?} has no existing ancestor inside plugin directory",
                path
            ));
        };
        missing.push(file_name.to_os_string());
        cursor = cursor.parent().ok_or_else(|| {
            format!(
                "Plugin module target {:?} has no existing ancestor inside plugin directory",
                path
            )
        })?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Scope;
    use tempfile::tempdir;

    fn engine_with_resolver(base_dir: PathBuf) -> Engine {
        let mut engine = Engine::new();
        engine.set_module_resolver(GuardedFileModuleResolver::new(base_dir));
        engine
    }

    #[test]
    fn nested_module_import_is_allowed() {
        let dir = tempdir().expect("tempdir");
        let scripts = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts).expect("mkdir scripts");
        std::fs::write(scripts.join("helper.rhai"), "fn value() { 42 }").expect("write helper");
        let engine = engine_with_resolver(dir.path().to_path_buf());
        let mut scope = Scope::new();

        let result = engine
            .eval_with_scope::<i64>(
                &mut scope,
                r#"
                    import "scripts/helper" as helper;
                    helper::value()
                "#,
            )
            .expect("nested import");

        assert_eq!(result, 42);
    }

    #[test]
    fn parent_traversal_import_is_rejected() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join("outside.rhai"), "fn value() { 1 }").expect("write outside");
        let plugin = dir.path().join("plugin");
        std::fs::create_dir(&plugin).expect("mkdir plugin");
        let engine = engine_with_resolver(plugin);

        let err = engine
            .eval::<i64>(
                r#"
                    import "../outside" as outside;
                    outside::value()
                "#,
            )
            .expect_err("parent traversal import must fail");

        assert!(err.to_string().contains("Invalid plugin module import"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_module_escape_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let plugin = dir.path().join("plugin");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&plugin).expect("mkdir plugin");
        std::fs::create_dir_all(&outside).expect("mkdir outside");
        std::fs::write(outside.join("helper.rhai"), "fn value() { 1 }")
            .expect("write outside helper");
        std::os::unix::fs::symlink(&outside, plugin.join("linked")).expect("symlink linked");
        let engine = engine_with_resolver(plugin);

        let err = engine
            .eval::<i64>(
                r#"
                    import "linked/helper" as helper;
                    helper::value()
                "#,
            )
            .expect_err("symlink module escape must fail");

        assert!(
            err.to_string()
                .contains("resolved module must stay inside plugin directory")
        );
    }
}
