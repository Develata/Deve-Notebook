//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use crate::plugin::resource_budget::{MAX_PLUGIN_SCRIPT_BYTES, read_utf8_file_bounded};
use rhai::{Engine, GlobalRuntimeState, Module, ModuleResolver, Position, Scope, Shared};
use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

/// File-backed Rhai module resolver constrained to one plugin directory.
#[derive(Debug)]
pub(super) struct GuardedFileModuleResolver {
    base_dir: PathBuf,
    cache: Mutex<BTreeMap<PathBuf, Shared<Module>>>,
    in_flight: Mutex<BTreeSet<PathBuf>>,
}

impl GuardedFileModuleResolver {
    pub(super) fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            cache: Mutex::new(BTreeMap::new()),
            in_flight: Mutex::new(BTreeSet::new()),
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
        if looks_like_windows_drive_path(path) {
            return Err(format!(
                "Invalid plugin module import '{}': drive prefixes are not allowed",
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

    fn guard_resolved_target(&self, path: &str) -> Result<PathBuf, String> {
        self.validate_module_path(path)?;

        let canonical_root = std::fs::canonicalize(&self.base_dir).map_err(|err| {
            format!(
                "Failed to canonicalize plugin module directory {:?}: {}",
                self.base_dir, err
            )
        })?;
        let mut target = self.base_dir.join(path);
        target.set_extension("rhai");
        let canonical_target = canonicalize_existing_ancestor(&target)?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(format!(
                "Invalid plugin module import '{}': resolved module must stay inside plugin directory",
                path
            ));
        }
        Ok(canonical_target)
    }

    fn resolve_with_runtime(
        &self,
        engine: &Engine,
        global: &mut GlobalRuntimeState,
        scope: &mut Scope,
        path: &str,
    ) -> Result<Shared<Module>, Box<rhai::EvalAltResult>> {
        let target = self
            .guard_resolved_target(path)
            .map_err(|err| -> Box<rhai::EvalAltResult> { err.into() })?;
        if let Some(module) = self
            .cache
            .lock()
            .map_err(|_| -> Box<rhai::EvalAltResult> { "Plugin module cache lock failed".into() })?
            .get(&target)
            .cloned()
        {
            return Ok(module);
        }
        let _in_flight = ModuleResolutionGuard::claim(&self.in_flight, target.clone(), path)?;

        let script = read_utf8_file_bounded(&target, MAX_PLUGIN_SCRIPT_BYTES, "plugin Rhai module")
            .map_err(|error| -> Box<rhai::EvalAltResult> {
                format!("Failed to read plugin module '{path}': {error}").into()
            })?;
        let mut ast = engine.compile(&script).map_err(|error| {
            Box::<rhai::EvalAltResult>::from(format!(
                "Failed to compile plugin module '{path}': {error}"
            ))
        })?;
        ast.set_source(path);
        let module: Shared<Module> = Module::eval_ast_as_new_raw(engine, scope, global, &ast)
            .map_err(|error| {
                Box::<rhai::EvalAltResult>::from(format!(
                    "Failed to initialize plugin module '{path}': {error}"
                ))
            })?
            .into();
        self.cache
            .lock()
            .map_err(|_| -> Box<rhai::EvalAltResult> { "Plugin module cache lock failed".into() })?
            .insert(target, module.clone());
        Ok(module)
    }
}

impl ModuleResolver for GuardedFileModuleResolver {
    fn resolve_raw(
        &self,
        engine: &Engine,
        global: &mut GlobalRuntimeState,
        scope: &mut Scope,
        path: &str,
        _pos: Position,
    ) -> Result<Shared<Module>, Box<rhai::EvalAltResult>> {
        self.resolve_with_runtime(engine, global, scope, path)
    }

    fn resolve(
        &self,
        engine: &Engine,
        source: Option<&str>,
        path: &str,
        pos: Position,
    ) -> Result<Shared<Module>, Box<rhai::EvalAltResult>> {
        let _ = (source, pos);
        let mut global = engine.new_global_runtime_state();
        self.resolve_with_runtime(engine, &mut global, &mut Scope::new(), path)
    }
}

struct ModuleResolutionGuard<'a> {
    in_flight: &'a Mutex<BTreeSet<PathBuf>>,
    target: PathBuf,
}

impl<'a> ModuleResolutionGuard<'a> {
    fn claim(
        in_flight: &'a Mutex<BTreeSet<PathBuf>>,
        target: PathBuf,
        import_path: &str,
    ) -> Result<Self, Box<rhai::EvalAltResult>> {
        let mut active = in_flight.lock().map_err(|_| -> Box<rhai::EvalAltResult> {
            "Plugin module cycle guard lock failed".into()
        })?;
        if !active.insert(target.clone()) {
            return Err(format!("Cyclic plugin module import rejected: '{import_path}'").into());
        }
        Ok(Self { in_flight, target })
    }
}

impl Drop for ModuleResolutionGuard<'_> {
    fn drop(&mut self) {
        match self.in_flight.lock() {
            Ok(mut active) => {
                active.remove(&self.target);
            }
            Err(poisoned) => {
                poisoned.into_inner().remove(&self.target);
            }
        }
    }
}

fn looks_like_windows_drive_path(path: &str) -> bool {
    matches!(
        path.as_bytes(),
        [drive, b':', ..] if drive.is_ascii_alphabetic()
    )
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

    #[test]
    fn windows_drive_prefix_import_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let engine = engine_with_resolver(dir.path().to_path_buf());

        let err = engine
            .eval::<i64>(
                r#"
                    import "C:/outside" as outside;
                    outside::value()
                "#,
            )
            .expect_err("drive-prefixed module import must fail");

        assert!(err.to_string().contains("drive prefixes are not allowed"));
    }

    #[test]
    fn oversized_module_is_rejected_before_evaluation() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("large.rhai"),
            vec![b' '; (MAX_PLUGIN_SCRIPT_BYTES + 1) as usize],
        )
        .expect("write large module");
        let engine = engine_with_resolver(dir.path().to_path_buf());

        let err = engine
            .eval::<()>(r#"import "large" as large;"#)
            .expect_err("oversized module must fail closed");

        assert!(err.to_string().contains("resource budget"));
    }

    #[test]
    fn cyclic_module_import_is_rejected_before_recursive_evaluation() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("first.rhai"),
            r#"import "second" as second; fn value() { second::value() }"#,
        )
        .expect("write first");
        std::fs::write(
            dir.path().join("second.rhai"),
            r#"import "first" as first; fn value() { first::value() }"#,
        )
        .expect("write second");
        let engine = engine_with_resolver(dir.path().to_path_buf());

        let error = engine
            .eval::<()>(r#"import "first" as first;"#)
            .expect_err("cyclic import must fail closed");

        assert!(
            error
                .to_string()
                .contains("Cyclic plugin module import rejected")
        );
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
