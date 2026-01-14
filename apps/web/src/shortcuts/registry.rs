//! # 快捷键注册表 (Shortcut Registry)
//!
//! 管理快捷键的注册、查询和冲突检测。

use std::collections::HashMap;
use super::types::{KeyCombo, Shortcut, ShortcutId};

/// 冲突错误
#[derive(Debug)]
pub struct ConflictError {
    pub existing_id: ShortcutId,
    pub combo: KeyCombo,
}

impl std::fmt::Display for ConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "快捷键冲突: {} 已被 {} 使用",
            self.combo, self.existing_id
        )
    }
}

/// 快捷键注册表
/// 
/// 存储所有已注册的快捷键，支持冲突检测。
#[derive(Debug, Default)]
pub struct ShortcutRegistry {
    /// 按 ID 索引的快捷键
    shortcuts: HashMap<String, Shortcut>,
    /// 按按键组合索引，用于快速查找
    combo_index: HashMap<String, ShortcutId>,
}

impl ShortcutRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成按键组合的唯一键
    fn combo_key(combo: &KeyCombo) -> String {
        format!(
            "{}:{}:{}:{}",
            combo.key,
            combo.ctrl as u8,
            combo.shift as u8,
            combo.alt as u8
        )
    }

    /// 注册快捷键
    /// 
    /// 如果快捷键已存在或按键组合冲突，返回错误。
    pub fn register(&mut self, shortcut: Shortcut) -> Result<(), ConflictError> {
        let combo_key = Self::combo_key(&shortcut.combo);

        // 检查冲突
        if let Some(existing_id) = self.combo_index.get(&combo_key) {
            return Err(ConflictError {
                existing_id: existing_id.clone(),
                combo: shortcut.combo.clone(),
            });
        }

        // 注册
        let id = shortcut.id.0.to_string();
        self.combo_index.insert(combo_key, shortcut.id.clone());
        self.shortcuts.insert(id, shortcut);
        Ok(())
    }

    /// 根据 ID 获取快捷键
    pub fn get(&self, id: &str) -> Option<&Shortcut> {
        self.shortcuts.get(id)
    }

    /// 根据按键组合查找快捷键
    pub fn find_by_combo(&self, combo: &KeyCombo) -> Option<&Shortcut> {
        let key = Self::combo_key(combo);
        self.combo_index
            .get(&key)
            .and_then(|id| self.shortcuts.get(id.0))
    }

    /// 获取所有快捷键
    pub fn all(&self) -> impl Iterator<Item = &Shortcut> {
        self.shortcuts.values()
    }

    /// 检查按键组合是否已注册
    pub fn has_combo(&self, combo: &KeyCombo) -> bool {
        let key = Self::combo_key(combo);
        self.combo_index.contains_key(&key)
    }
}
