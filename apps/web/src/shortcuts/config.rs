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
use serde::{Deserialize, Serialize};
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

        Self::from_json(&data).unwrap_or_default()
    }

    /// 保存配置到 UI prefs fallback 层。
    pub fn save(&self) -> Result<(), JsValue> {
        let json = self
            .to_json()
            .map_err(|err| JsValue::from_str(&format!("serialize shortcut prefs: {err}")))?;
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

    fn from_json(data: &str) -> Option<Self> {
        if let Ok(overrides) = serde_json::from_str::<HashMap<String, StoredKeyCombo>>(data) {
            return Some(Self {
                overrides: overrides
                    .into_iter()
                    .map(|(id, combo)| (id, combo.into_key_combo()))
                    .collect(),
            });
        }
        Self::from_legacy_json(data)
    }

    fn from_legacy_json(data: &str) -> Option<Self> {
        // Legacy prefs used {"id":"key:ctrl:shift:alt"}. New writes use StoredKeyCombo.
        let legacy = serde_json::from_str::<HashMap<String, String>>(data).ok()?;
        let overrides = legacy
            .into_iter()
            .filter_map(|(id, combo)| parse_legacy_combo(&combo).map(|combo| (id, combo)))
            .collect();
        Some(Self { overrides })
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        let overrides = self
            .overrides
            .iter()
            .map(|(id, combo)| (id.clone(), StoredKeyCombo::from(combo)))
            .collect::<HashMap<_, _>>();
        serde_json::to_string(&overrides)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredKeyCombo {
    key: String,
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl StoredKeyCombo {
    fn into_key_combo(self) -> KeyCombo {
        KeyCombo::new(&self.key, self.ctrl, self.shift, self.alt)
    }
}

impl From<&KeyCombo> for StoredKeyCombo {
    fn from(combo: &KeyCombo) -> Self {
        Self {
            key: combo.key.clone(),
            ctrl: combo.ctrl,
            shift: combo.shift,
            alt: combo.alt,
        }
    }
}

fn parse_legacy_combo(value: &str) -> Option<KeyCombo> {
    let mut parts = value.split(':');
    let key = parts.next()?;
    let ctrl = parse_legacy_bool(parts.next()?)?;
    let shift = parse_legacy_bool(parts.next()?)?;
    let alt = parse_legacy_bool(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(KeyCombo::new(key, ctrl, shift, alt))
}

fn parse_legacy_bool(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::prefs::{remove_pref, write_pref};

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

    #[test]
    fn shortcut_config_uses_structured_json_for_new_writes() {
        let mut config = ShortcutConfig::new();
        config.set_override("open:palette", KeyCombo::new(":", true, true, true));

        let json = config.to_json().expect("shortcut prefs json");
        let value: serde_json::Value = serde_json::from_str(&json).expect("json value");

        assert_eq!(value["open:palette"]["key"], ":");
        assert_eq!(value["open:palette"]["ctrl"], true);
        assert_eq!(value["open:palette"]["shift"], true);
        assert_eq!(value["open:palette"]["alt"], true);
    }

    #[test]
    fn shortcut_config_roundtrips_escaped_ids_and_keys() {
        remove_pref(STORAGE_KEY);
        let mut config = ShortcutConfig::new();
        config.set_override("open,\"quote\"", KeyCombo::new("\\", true, false, true));

        config.save().expect("save shortcut prefs");
        let loaded = ShortcutConfig::load();

        assert_eq!(
            loaded.get_override("open,\"quote\""),
            Some(&KeyCombo::new("\\", true, false, true))
        );
        remove_pref(STORAGE_KEY);
    }

    #[test]
    fn shortcut_config_reads_legacy_delimited_prefs() {
        remove_pref(STORAGE_KEY);
        write_pref(STORAGE_KEY, r#"{"open":"p:1:0:1"}"#).expect("write legacy prefs");

        let loaded = ShortcutConfig::load();

        assert_eq!(
            loaded.get_override("open"),
            Some(&KeyCombo::new("p", true, false, true))
        );
        remove_pref(STORAGE_KEY);
    }
}
