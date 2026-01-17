// crates\core\src\plugin
//! # Plugin Runtime (插件运行时)
//!
//! **架构作用**:
//! 管理插件的加载、执行环境与生命周期。
//!
//! **核心功能清单**:
//! - `PluginRuntime`: trait 接口，定义加载与执行标准。
//! - `RhaiRuntime`: 基于 Rhai 脚本引擎的实现。
//! - `Host Functions`: 注入宿主能力（后续实现）。
//!
//! **类型**: Core MUST (核心必选)

use crate::plugin::manifest::PluginManifest;
use anyhow::{Result, anyhow};
use rhai::{Engine, Scope, AST};
use std::sync::{Arc, Mutex};

/// Abstract interface for a plugin runtime.
pub trait PluginRuntime: Send + Sync {
    /// Load a plugin from a script source.
    fn load(&mut self, manifest: PluginManifest, script: &str) -> Result<()>;
    
    /// Execute a specific function in the plugin.
    fn call(&self, fn_name: &str, args: Vec<rhai::Dynamic>) -> Result<rhai::Dynamic>;

    /// Get the manifest of the loaded plugin.
    fn manifest(&self) -> &PluginManifest;
}

/// A runtime implementation using the Rhai scripting engine.
pub struct RhaiRuntime {
    engine: Engine,
    ast: Option<AST>,
    scope: Mutex<Scope<'static>>,
    manifest: PluginManifest,
}

impl RhaiRuntime {
    pub fn new(manifest: PluginManifest) -> Self {
        let mut engine = Engine::new();
        
        // Register standard library / host functions
        Self::register_core_api(&mut engine, &manifest);

        Self {
            engine,
            ast: None,
            scope: Mutex::new(Scope::new()),
            manifest,
        }
    }

    fn register_core_api(engine: &mut Engine, manifest: &PluginManifest) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let caps = Arc::new(manifest.capabilities.clone());
            
            // fs_read(path: &str) -> String
            engine.register_fn("fs_read", move |path: &str| -> Result<String, Box<rhai::EvalAltResult>> {
                use std::path::Path;
                let p = Path::new(path);
                if !caps.clone().check_read(p) {
                    return Err(format!("Permission denied: read access to '{}' is not allowed by manifest.", path).into());
                }
                std::fs::read_to_string(p).map_err(|e| format!("IO Error: {}", e).into())
            });

            let caps_write = Arc::new(manifest.capabilities.clone());
            // fs_write(path: &str, content: &str)
            engine.register_fn("fs_write", move |path: &str, content: &str| -> Result<(), Box<rhai::EvalAltResult>> {
                use std::path::Path;
                let p = Path::new(path);
                if !caps_write.check_write(p) {
                    return Err(format!("Permission denied: write access to '{}' is not allowed by manifest.", path).into());
                }
                std::fs::write(p, content).map_err(|e| format!("IO Error: {}", e).into())
            });
        }
        
        // Universal Functions (Wasm + Native)
        engine.register_fn("log_info", |msg: &str| {
            println!("[Plugin Log] {}", msg);
            // In a real app we might use tracing::info!
        });
    }
}

impl PluginRuntime for RhaiRuntime {
    fn load(&mut self, _manifest: PluginManifest, script: &str) -> Result<()> {
        // Compile the script into an AST.
        let ast = self.engine.compile(script).map_err(|e| anyhow!("Failed to compile plugin script: {}", e))?;
        
        // Run the script once to initialize global state/variables.
        // We need to lock the scope.
        let mut scope = self.scope.lock().map_err(|_| anyhow!("Failed to lock plugin scope"))?;
        self.engine.run_ast_with_scope(&mut *scope, &ast)
             .map_err(|e| anyhow!("Failed to initialize plugin: {}", e))?;

        self.ast = Some(ast);
        Ok(())
    }

    fn call(&self, fn_name: &str, args: Vec<rhai::Dynamic>) -> Result<rhai::Dynamic> {
        let ast = self.ast.as_ref().ok_or_else(|| anyhow!("Plugin not loaded"))?;
        
        // Lock the scope effectively making it single-threaded execution per plugin instance (which is fine for safety)
        let mut scope = self.scope.lock().map_err(|_| anyhow!("Failed to lock plugin scope"))?;
        
        self.engine.call_fn(&mut *scope, ast, fn_name, args)
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

    #[test]
    fn test_rhai_basic_execution() {
        let manifest = PluginManifest {
            id: "test_plugin".into(),
            name: "Test".into(),
            version: "0.1.0".into(),
            entry: "main.rhai".into(),
            capabilities: Capability::default(),
        };

        let mut runtime = RhaiRuntime::new(manifest.clone());
        let script = r#"
            let x = 10;
            
            fn add(a, b) {
                return a + b + x;
            }
        "#;

        runtime.load(manifest, script).expect("Failed to load script");
        
        let args = vec![5.into(), 5.into()];
        let result = runtime.call("add", args).expect("Failed to call fn");
        assert_eq!(result.as_int().unwrap(), 20); // 5 + 5 + 10
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_rhai_fs_security() {
        // 1. Create a dummy file
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        write!(temp, "secret data").unwrap();
        let path = temp.path().to_str().unwrap().to_string();
        let path_buf = temp.path().to_path_buf();

        // 2. Plugin WITH permission
        {
            let mut cap = Capability::default();
            cap.allow_fs_read.push(path_buf.clone());
            
            let manifest = PluginManifest {
                id: "allowed".into(),
                name: "Allowed".into(),
                version: "0.1.0".into(),
                entry: "main.rhai".into(),
                capabilities: cap,
            };
            
            let mut runtime = RhaiRuntime::new(manifest.clone());
            let script = r#"
                fn read_it(p) {
                    return fs_read(p);
                }
            "#;
            runtime.load(manifest, script).unwrap();
            
            let res = runtime.call("read_it", vec![path.clone().into()]).unwrap();
            assert_eq!(res.into_string().unwrap(), "secret data");
        }

        // 3. Plugin WITHOUT permission
        {
            let cap = Capability::default(); // Empty caps
             let manifest = PluginManifest {
                id: "denied".into(),
                name: "Denied".into(),
                version: "0.1.0".into(),
                entry: "main.rhai".into(),
                capabilities: cap,
            };

            let mut runtime = RhaiRuntime::new(manifest.clone());
            let script = r#"
                fn read_it(p) {
                    return fs_read(p);
                }
            "#;
            runtime.load(manifest, script).unwrap();
            
            let res = runtime.call("read_it", vec![path.clone().into()]);
            assert!(res.is_err());
            let err_msg = res.unwrap_err().to_string();
            assert!(err_msg.contains("Permission denied"));
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_rhai_fs_write() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("output.txt");
        let path_str = file_path.to_str().unwrap().to_string();

        let mut cap = Capability::default();
        cap.allow_fs_write.push(file_path.clone());

        let manifest = PluginManifest {
            id: "writer".into(),
            name: "Writer".into(),
            version: "0.1.0".into(),
            entry: "main.rhai".into(),
            capabilities: cap,
        };

        let mut runtime = RhaiRuntime::new(manifest.clone());
        let script = r#"
            fn write_test(p, c) {
                log_info("Writing file...");
                fs_write(p, c);
            }
        "#;
        runtime.load(manifest, script).unwrap();

        runtime.call("write_test", vec![path_str.into(), "Hello World".into()]).expect("Call failed");

        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "Hello World");
    }
}
