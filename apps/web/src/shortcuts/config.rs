//! # 快捷键配置 (Shortcut Config)
//!
//! 用户自定义快捷键配置，支持 localStorage 持久化。

use std::collections::HashMap;
use super::types::KeyCombo;
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

    /// 从 localStorage 加载配置
    pub fn load() -> Self {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return Self::default(),
        };

        let storage = match window.local_storage() {
            Ok(Some(s)) => s,
            _ => return Self::default(),
        };

        let data = match storage.get_item(STORAGE_KEY) {
            Ok(Some(d)) => d,
            _ => return Self::default(),
        };

        // 解析 JSON
        Self::parse_json(&data).unwrap_or_default()
    }

    /// 保存配置到 localStorage
    pub fn save(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let storage = window
            .local_storage()?
            .ok_or("no localStorage")?;

        let json = self.to_json();
        storage.set_item(STORAGE_KEY, &json)?;
        Ok(())
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

                config.overrides.insert(
                    id.to_string(),
                    KeyCombo::new(key, ctrl, shift, alt),
                );
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
                    id,
                    combo.key,
                    combo.ctrl as u8,
                    combo.shift as u8,
                    combo.alt as u8
                )
            })
            .collect();
        format!("{{{}}}", pairs.join(","))
    }
}
