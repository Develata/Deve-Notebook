// crates\core\src\plugin\runtime\rhai_v1.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # Rhai Runtime Implementation
//!
//! **功能**:
//! 基于 Rhai 脚本引擎实现的插件运行时。
//!
//! **模块化支持**:
//! 通过 FileModuleResolver 支持 `import "module_name"` 语法，
//! 允许插件拆分为多个 .rhai 文件。(仅非 WASM 环境)

use super::{PluginRuntime, host};
use crate::plugin::manifest::PluginManifest;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugin::runtime::module_resolver::GuardedFileModuleResolver;
use anyhow::{Result, anyhow};
#[cfg(not(target_arch = "wasm32"))]
use rhai::EvalAltResult;
use rhai::{AST, Dynamic, Engine, Scope};
use std::path::PathBuf;
use std::sync::Mutex;

/// Rhai 脚本最大操作数限制，防止无限循环 (768 MB 内存安全阈值)
const MAX_RHAI_OPERATIONS: u64 = 100_000;

/// Rhai 引擎运行时
///
/// **Invariant**: `base_dir` 必须是插件根目录的有效路径。
/// **Post-condition**: 引擎配置了 FileModuleResolver，可解析同目录下的 .rhai 模块。
pub struct RhaiRuntime {
    engine: Engine,
    ast: Option<AST>,
    scope: Mutex<Scope<'static>>,
    manifest: PluginManifest,
}

impl RhaiRuntime {
    /// 创建新的运行时实例
    ///
    /// **参数**:
    /// - `manifest`: 插件清单
    /// - `_base_dir`: 插件根目录路径，用于解析非 WASM 目标的 import 语句
    pub fn new(manifest: PluginManifest, _base_dir: PathBuf) -> Self {
        Self::new_with_module_root(manifest, Some(_base_dir), false)
    }

    /// Create a runtime for a compile-time embedded script.
    ///
    /// Embedded first-party scripts have no filesystem module resolver, so an
    /// `import` added later cannot silently turn the binary asset back into a
    /// runtime-directory dependency.
    pub fn new_embedded(manifest: PluginManifest) -> Self {
        Self::new_with_module_root(manifest, None, false)
    }

    /// Creates the compile-time first-party Native AI compatibility runtime.
    /// This is the only constructor that exposes the server-owned chat stream
    /// bridge; filesystem-backed plugins remain default-deny.
    pub fn new_embedded_native_ai(manifest: PluginManifest) -> Self {
        Self::new_with_module_root(manifest, None, true)
    }

    fn new_with_module_root(
        manifest: PluginManifest,
        _base_dir: Option<PathBuf>,
        allow_native_ai_stream: bool,
    ) -> Self {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(128, 128);
        engine.set_max_operations(MAX_RHAI_OPERATIONS);
        engine.disable_symbol("eval");

        // 配置模块解析器 (仅非 WASM 环境支持文件系统)
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(base_dir) = _base_dir {
            let resolver = GuardedFileModuleResolver::new(base_dir);
            engine.set_module_resolver(resolver);
        }

        // 注册宿主 API
        host::register_core_api_with_native_ai(&mut engine, &manifest, allow_native_ai_stream);

        Self {
            engine,
            ast: None,
            scope: Mutex::new(Scope::new()),
            manifest,
        }
    }
}

impl PluginRuntime for RhaiRuntime {
    fn load(&mut self, _manifest: PluginManifest, script: &str) -> Result<()> {
        // 编译脚本为 AST
        let ast = self
            .engine
            .compile(script)
            .map_err(|e| anyhow!("Failed to compile plugin script: {}", e))?;

        // 初始化全局状态
        let mut scope = self
            .scope
            .lock()
            .map_err(|_| anyhow!("Failed to lock plugin scope"))?;

        self.engine
            .run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| anyhow!("Failed to initialize plugin: {}", e))?;

        self.ast = Some(ast);
        Ok(())
    }

    fn call(&self, fn_name: &str, args: Vec<Dynamic>) -> Result<Dynamic> {
        let ast = self
            .ast
            .as_ref()
            .ok_or_else(|| anyhow!("Plugin not loaded"))?;

        // 警告: 持有 Scope 锁期间阻止重入 (Reentrancy)
        let mut scope = self
            .scope
            .lock()
            .map_err(|_| anyhow!("Failed to lock plugin scope"))?;

        match self.engine.call_fn(&mut scope, ast, fn_name, args) {
            Ok(result) => Ok(result),
            Err(error) => {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(server_error) = plugin_host_server_error(&error) {
                    return Err(anyhow::Error::new(server_error.clone()));
                }
                Err(anyhow!(
                    "Runtime error in function '{}': {}",
                    fn_name,
                    error
                ))
            }
        }
    }

    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn plugin_host_server_error(error: &EvalAltResult) -> Option<&host::PluginHostServerError> {
    match error {
        EvalAltResult::ErrorSystem(_, source) => {
            source.downcast_ref::<host::PluginHostServerError>()
        }
        EvalAltResult::ErrorInFunctionCall(_, _, inner, _)
        | EvalAltResult::ErrorInModule(_, inner, _) => plugin_host_server_error(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manifest::Capability;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn test_rhai_basic_execution() {
        let manifest = PluginManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "0.1".into(),
            entry: "main.rhai".into(),
            capabilities: Default::default(),
        };
        let base_dir = PathBuf::from(".");
        let mut runtime = RhaiRuntime::new(manifest.clone(), base_dir);
        runtime
            .load(manifest.clone(), "fn add(a, b) { a + b }")
            .unwrap();
        let res = runtime.call("add", vec![1.into(), 2.into()]).unwrap();
        assert_eq!(res.as_int().unwrap(), 3);
    }

    #[test]
    fn embedded_rhai_runtime_does_not_resolve_filesystem_modules() {
        let manifest = PluginManifest {
            id: "embedded-test".into(),
            name: "Embedded Test".into(),
            version: "0.1".into(),
            entry: "main.rhai".into(),
            capabilities: Default::default(),
        };
        let mut runtime = RhaiRuntime::new_embedded(manifest.clone());
        let error = runtime
            .load(manifest, "import \"missing\" as missing;")
            .expect_err("embedded runtime must not resolve filesystem modules");

        assert!(error.to_string().contains("Failed to initialize plugin"));
    }

    #[test]
    fn external_rhai_runtime_has_no_native_ai_stream_authority() {
        let manifest = PluginManifest {
            id: "external-ai-probe".into(),
            name: "External AI Probe".into(),
            version: "0.1".into(),
            entry: "main.rhai".into(),
            capabilities: Default::default(),
        };
        let mut runtime = RhaiRuntime::new(manifest.clone(), PathBuf::from("."));
        runtime
            .load(manifest, r#"fn run() { ai_chat_stream("req", []) }"#)
            .expect("unresolved host names may compile inside a function");

        let error = runtime
            .call("run", Vec::new())
            .expect_err("external plugin must not receive Native AI stream API");
        assert!(error.to_string().contains("Function not found"));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_rhai_security() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        write!(temp, "secret").unwrap();
        let path = temp.path().to_path_buf();
        let path_str = path.to_str().unwrap().to_string();
        let base_dir = PathBuf::from(".");

        // Allowed
        let mut cap = Capability::default();
        cap.allow_fs_read.push(path.clone());
        let manifest = PluginManifest {
            id: "ok".into(),
            name: "OK".into(),
            version: "0.1".into(),
            entry: "m.rhai".into(),
            capabilities: cap,
        };
        let mut rt = RhaiRuntime::new(manifest.clone(), base_dir.clone());
        rt.load(manifest, "fn read(p) { fs_read(p) }").unwrap();
        assert_eq!(
            rt.call("read", vec![path_str.clone().into()])
                .unwrap()
                .into_string()
                .unwrap(),
            "secret"
        );

        // Denied
        let manifest_deny = PluginManifest {
            id: "deny".into(),
            name: "Deny".into(),
            version: "0.1".into(),
            entry: "m.rhai".into(),
            capabilities: Default::default(),
        };
        let mut rt_deny = RhaiRuntime::new(manifest_deny.clone(), base_dir);
        rt_deny
            .load(manifest_deny, "fn read(p) { fs_read(p) }")
            .unwrap();
        assert!(rt_deny.call("read", vec![path_str.into()]).is_err());
    }

    #[test]
    fn rhai_eval_symbol_is_disabled() {
        let manifest = PluginManifest {
            id: "eval-denied".into(),
            name: "Eval Denied".into(),
            version: "0.1".into(),
            entry: "m.rhai".into(),
            capabilities: Default::default(),
        };
        let mut rt = RhaiRuntime::new(manifest.clone(), PathBuf::from("."));

        assert!(rt.load(manifest, r#"fn run() { eval("40 + 2") }"#).is_err());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn nested_rhai_error_preserves_typed_plugin_host_server_error() {
        let server_error = crate::protocol::ServerError::workspace_ingestion_unavailable();
        let nested = EvalAltResult::ErrorInFunctionCall(
            "writer".to_string(),
            "test".to_string(),
            Box::new(EvalAltResult::ErrorSystem(
                "host".to_string(),
                Box::new(host::PluginHostServerError(server_error.clone())),
            )),
            rhai::Position::NONE,
        );

        assert_eq!(
            plugin_host_server_error(&nested).map(|error| error.0.clone()),
            Some(server_error)
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn rhai_call_returns_typed_plugin_host_server_error() {
        let manifest = PluginManifest {
            id: "typed-host-error".into(),
            name: "Typed Host Error".into(),
            version: "0.1".into(),
            entry: "main.rhai".into(),
            capabilities: Default::default(),
        };
        let mut runtime = RhaiRuntime::new(manifest.clone(), PathBuf::from("."));
        runtime
            .engine
            .register_fn("typed_fail", || -> Result<(), Box<EvalAltResult>> {
                Err(host::server_error_to_eval(
                    crate::protocol::ServerError::workspace_ingestion_unavailable(),
                ))
            });
        runtime
            .load(manifest, "fn run() { typed_fail(); }")
            .expect("script loads");

        let error = runtime.call("run", Vec::new()).expect_err("typed failure");
        let typed = error
            .downcast_ref::<host::PluginHostServerError>()
            .expect("typed host error survives Rhai call");
        assert_eq!(
            typed.0,
            crate::protocol::ServerError::workspace_ingestion_unavailable()
        );
    }
}
