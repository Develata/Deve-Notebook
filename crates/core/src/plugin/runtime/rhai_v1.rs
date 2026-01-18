// crates\core\src\plugin\runtime\rhai_v1.rs
//! # Rhai Runtime Implementation
//!
//! **功能**:
//! 基于 Rhai 脚本引擎实现的插件运行时。

use super::{PluginRuntime, host};
use crate::plugin::manifest::PluginManifest;
use anyhow::{Result, anyhow};
use rhai::{AST, Dynamic, Engine, Scope};
use std::sync::Mutex;

/// Rhai 引擎运行时
pub struct RhaiRuntime {
    engine: Engine,
    ast: Option<AST>,
    scope: Mutex<Scope<'static>>,
    manifest: PluginManifest,
}

impl RhaiRuntime {
    /// 创建新的运行时实例
    pub fn new(manifest: PluginManifest) -> Self {
        let mut engine = Engine::new();

        // 注册宿主 API
        host::register_core_api(&mut engine, &manifest);

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
            .run_ast_with_scope(&mut *scope, &ast)
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

        self.engine
            .call_fn(&mut *scope, ast, fn_name, args)
            .map_err(|e| anyhow!("Runtime error in function '{}': {}", fn_name, e))
    }

    fn manifest(&self) -> &PluginManifest {
        &self.manifest
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
        let mut runtime = RhaiRuntime::new(manifest.clone());
        runtime
            .load(manifest.clone(), "fn add(a, b) { a + b }")
            .unwrap();
        let res = runtime.call("add", vec![1.into(), 2.into()]).unwrap();
        assert_eq!(res.as_int().unwrap(), 3);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_rhai_security() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        write!(temp, "secret").unwrap();
        let path = temp.path().to_path_buf();
        let path_str = path.to_str().unwrap().to_string();

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
        let mut rt = RhaiRuntime::new(manifest.clone());
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
        let mut rt_deny = RhaiRuntime::new(manifest_deny.clone());
        rt_deny
            .load(manifest_deny, "fn read(p) { fs_read(p) }")
            .unwrap();
        assert!(rt_deny.call("read", vec![path_str.into()]).is_err());
    }
}
