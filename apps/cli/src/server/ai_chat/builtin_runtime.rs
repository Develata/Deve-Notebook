//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Compile-time Native AI runtime assembly.

use anyhow::{Context, bail};
use deve_core::plugin::manifest::PluginManifest;
use deve_core::plugin::runtime::{PluginRuntime, RhaiRuntime};
use std::collections::HashSet;

pub(super) const NATIVE_AI_PLUGIN_ID: &str = "ai-chat";
const BUILTIN_MANIFEST: &str = include_str!("../../../../../plugins/ai-chat/manifest.json");
const BUILTIN_SCRIPT: &str = include_str!("../../../../../plugins/ai-chat/main.rhai");

pub(super) fn assemble_runtime_plugins(
    external_plugins: Vec<Box<dyn PluginRuntime>>,
    native_ai_enabled: bool,
) -> anyhow::Result<Vec<Box<dyn PluginRuntime>>> {
    let mut seen = HashSet::new();
    for plugin in &external_plugins {
        let id = plugin.manifest().id.as_str();
        if !seen.insert(id.to_string()) {
            bail!("Duplicate plugin id '{id}'");
        }
    }

    if !native_ai_enabled {
        return Ok(external_plugins
            .into_iter()
            .filter(|plugin| plugin.manifest().id != NATIVE_AI_PLUGIN_ID)
            .collect());
    }
    if seen.contains(NATIVE_AI_PLUGIN_ID) {
        bail!("Duplicate plugin id '{NATIVE_AI_PLUGIN_ID}'");
    }

    let mut plugins = Vec::with_capacity(external_plugins.len() + 1);
    plugins.push(builtin_native_ai_runtime()?);
    plugins.extend(external_plugins);
    Ok(plugins)
}

fn builtin_native_ai_runtime() -> anyhow::Result<Box<dyn PluginRuntime>> {
    let manifest: PluginManifest =
        serde_json::from_str(BUILTIN_MANIFEST).context("Invalid built-in Native AI manifest")?;
    if manifest.id != NATIVE_AI_PLUGIN_ID || manifest.entry != "main.rhai" {
        bail!("Invalid built-in Native AI manifest identity");
    }
    let mut runtime = RhaiRuntime::new_embedded(manifest.clone());
    runtime
        .load(manifest, BUILTIN_SCRIPT)
        .context("Failed to load built-in Native AI runtime")?;
    Ok(Box::new(runtime))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rhai::Dynamic;

    struct DuplicateAiRuntime {
        manifest: PluginManifest,
    }

    impl DuplicateAiRuntime {
        fn new() -> Self {
            Self {
                manifest: serde_json::from_str(BUILTIN_MANIFEST).expect("manifest"),
            }
        }
    }

    impl PluginRuntime for DuplicateAiRuntime {
        fn load(&mut self, _manifest: PluginManifest, _script: &str) -> Result<()> {
            Ok(())
        }

        fn call(&self, _fn_name: &str, _args: Vec<Dynamic>) -> Result<Dynamic> {
            Ok(().into())
        }

        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
    }

    #[test]
    fn native_ai_builtin_registers_without_external_plugin_directory() {
        let plugins = assemble_runtime_plugins(Vec::new(), true).expect("built-in runtime");

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest().id, NATIVE_AI_PLUGIN_ID);
    }

    #[test]
    fn native_ai_builtin_duplicate_external_id_fails_closed() {
        let error = match assemble_runtime_plugins(vec![Box::new(DuplicateAiRuntime::new())], true)
        {
            Ok(_) => panic!("duplicate id must fail closed"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("Duplicate plugin id 'ai-chat'"));
    }

    #[test]
    fn native_ai_disabled_removes_compatibility_plugin_registration() {
        let plugins = assemble_runtime_plugins(vec![Box::new(DuplicateAiRuntime::new())], false)
            .expect("disabled native AI filters compatibility runtime");

        assert!(plugins.is_empty());
    }
}
