// apps\web\src\shortcuts
//! # 快捷键配置 (Shortcut Config)
//! plan_ref:
//!   - 15_settings#keyboard-shortcuts
//!   - 15_settings#browser-ui-prefs
//!
//! 用户自定义快捷键配置，支持 UI prefs fallback 持久化。

#![allow(dead_code)] // 快捷键系统模块预留

use super::types::KeyCombo;
use crate::storage::prefs::{read_pref, write_pref};
use std::collections::HashMap;
use wasm_bindgen::JsValue;

const STORAGE_KEY: &str = "deve_note_shortcuts";

/// 用户快捷键配置
///
/// 存储用户自定义的快捷键覆盖。
#[derive(Debug, Default, Clone)]
pub struct ShortcutConfig {
    /// 快捷键覆盖映射 (ID -> 新的按键组合)
    pub overrides: HashMap<String, KeyCombo>,
}

impl ShortcutConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 UI prefs fallback 层加载配置。
    pub fn load() -> Self {
        let data = match read_pref(STORAGE_KEY) {
            Some(data) => data,
            None => return Self::default(),
        };

        // 解析 JSON
        Self::parse_json(&data).unwrap_or_default()
    }

    /// 保存配置到 UI prefs fallback 层。
    pub fn save(&self) -> Result<(), JsValue> {
        let json = self.to_json();
        write_pref(STORAGE_KEY, &json).map_err(|err| JsValue::from_str(err.message()))
    }

    /// 设置快捷键覆盖
    pub fn set_override(&mut self, id: &str, combo: KeyCombo) {
        self.overrides.insert(id.to_string(), combo);
    }

    /// 移除快捷键覆盖
    pub fn remove_override(&mut self, id: &str) {
        self.overrides.remove(id);
    }

    /// 获取快捷键覆盖
    pub fn get_override(&self, id: &str) -> Option<&KeyCombo> {
        self.overrides.get(id)
    }

    /// 解析 JSON 字符串
    fn parse_json(data: &str) -> Option<Self> {
        // 简单的手动解析，避免引入 serde
        // 格式: {"id1":"key:ctrl:shift:alt", ...}
        let mut config = Self::new();

        let data = data.trim().trim_start_matches('{').trim_end_matches('}');
        for pair in data.split(',') {
            let parts: Vec<&str> = pair.split(':').collect();
            if parts.len() >= 5 {
                let id = parts[0].trim().trim_matches('"');
                let key = parts[1].trim().trim_matches('"');
                let ctrl = parts[2].trim() == "1";
                let shift = parts[3].trim() == "1";
                let alt = parts[4].trim().trim_matches('"') == "1";

                config
                    .overrides
                    .insert(id.to_string(), KeyCombo::new(key, ctrl, shift, alt));
            }
        }
        Some(config)
    }

    /// 转换为 JSON 字符串
    fn to_json(&self) -> String {
        let pairs: Vec<String> = self
            .overrides
            .iter()
            .map(|(id, combo)| {
                format!(
                    "\"{}\":\"{}:{}:{}:{}\"",
                    id, combo.key, combo.ctrl as u8, combo.shift as u8, combo.alt as u8
                )
            })
            .collect();
        format!("{{{}}}", pairs.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::prefs::remove_pref;

    #[test]
    fn shortcut_config_roundtrips_through_ui_prefs_layer() {
        remove_pref(STORAGE_KEY);
        let mut config = ShortcutConfig::new();
        config.set_override("open", KeyCombo::new("p", true, false, false));

        config.save().expect("save shortcut prefs");
        let loaded = ShortcutConfig::load();

        assert_eq!(
            loaded.get_override("open"),
            Some(&KeyCombo::new("p", true, false, false))
        );
        remove_pref(STORAGE_KEY);
    }
}
