// crates/core/src/plugin/runtime/host/util.rs
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # 辅助宿主函数
//!
//! **功能**: 提供 JSON 解析、环境变量读取和日志输出能力。

#[cfg(not(target_arch = "wasm32"))]
use crate::plugin::manifest::Capability;
use rhai::Engine;
#[cfg(not(target_arch = "wasm32"))]
use rhai::EvalAltResult;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

/// 注册辅助 API (需 Capability 检查) - 仅非 WASM
#[cfg(not(target_arch = "wasm32"))]
pub fn register_util_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_env = caps.clone();

    // API: to_json(val: Dynamic) -> String
    engine.register_fn(
        "to_json",
        |val: rhai::Dynamic| -> Result<String, Box<EvalAltResult>> {
            serde_json::to_string(&val).map_err(|e| e.to_string().into())
        },
    );

    // API: parse_json(json: &str) -> Dynamic
    engine.register_fn(
        "parse_json",
        |json: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            serde_json::from_str(json).map_err(|e| e.to_string().into())
        },
    );

    // API: env(key: &str) -> Dynamic (受 Capability 限制)
    engine.register_fn(
        "env",
        move |key: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_env.check_env(key) {
                return Err(format!(
                    "Permission denied: env access to '{}' is not allowed by manifest",
                    key
                )
                .into());
            }
            match std::env::var(key) {
                Ok(v) => Ok(v.into()),
                Err(_) => Ok(rhai::Dynamic::UNIT),
            }
        },
    );
}

/// 注册日志 API (跨平台，无需 Capability)
pub fn register_log_api(engine: &mut Engine) {
    engine.register_fn("log_info", |msg: &str| {
        println!("[Plugin Log] {}", msg);
    });
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn env_denies_unlisted_key() {
        let mut engine = Engine::new();
        register_util_api(&mut engine, Arc::new(Capability::default()));

        let err = engine
            .eval::<rhai::Dynamic>(r#"env("SECRET_KEY")"#)
            .expect_err("unlisted env key must fail closed");

        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn env_returns_unit_for_allowed_unset_key() {
        let _guard = ENV_TEST_LOCK.lock().expect("env test lock");
        let key = format!("DEVE_TEST_PLUGIN_ENV_UNSET_{}", std::process::id());
        // SAFETY: This test serializes access through ENV_TEST_LOCK and uses a
        // process-unique key that production code does not read.
        unsafe {
            std::env::remove_var(&key);
        }

        let mut cap = Capability::default();
        cap.allow_env.push(key.clone());
        let mut engine = Engine::new();
        register_util_api(&mut engine, Arc::new(cap));

        let value = engine
            .eval::<rhai::Dynamic>(&format!(r#"env("{}")"#, key))
            .expect("allowed unset env returns Unit");

        assert!(value.is_unit());
    }
}
