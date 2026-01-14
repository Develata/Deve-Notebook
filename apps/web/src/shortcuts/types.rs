//! # 快捷键类型定义 (Shortcut Types)
//!
//! 定义快捷键相关的核心类型。

use std::fmt;

/// 快捷键唯一标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShortcutId(pub &'static str);

impl fmt::Display for ShortcutId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 按键组合
/// 
/// 表示一个快捷键的按键组合，如 Ctrl+P, Ctrl+Shift+P 等。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    /// 主键 (小写，如 "p", "k", "enter")
    pub key: String,
    /// 是否按下 Ctrl 键 (Windows/Linux) 或 Command 键 (macOS)
    pub ctrl: bool,
    /// 是否按下 Shift 键
    pub shift: bool,
    /// 是否按下 Alt 键
    pub alt: bool,
}

impl KeyCombo {
    /// 创建新的按键组合
    pub fn new(key: &str, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self {
            key: key.to_lowercase(),
            ctrl,
            shift,
            alt,
        }
    }

    /// 从键盘事件创建按键组合
    pub fn from_event(ev: &web_sys::KeyboardEvent) -> Self {
        Self {
            key: ev.key().to_lowercase(),
            ctrl: ev.ctrl_key() || ev.meta_key(), // 兼容 macOS
            shift: ev.shift_key(),
            alt: ev.alt_key(),
        }
    }

    /// 检查是否匹配键盘事件
    pub fn matches(&self, ev: &web_sys::KeyboardEvent) -> bool {
        let is_ctrl = ev.ctrl_key() || ev.meta_key();
        ev.key().to_lowercase() == self.key
            && is_ctrl == self.ctrl
            && ev.shift_key() == self.shift
            && ev.alt_key() == self.alt
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.shift { parts.push("Shift"); }
        if self.alt { parts.push("Alt"); }
        let key_upper = self.key.to_uppercase();
        parts.push(&key_upper);
        write!(f, "{}", parts.join("+"))
    }
}

/// 快捷键定义
#[derive(Debug, Clone)]
pub struct Shortcut {
    /// 唯一标识符
    pub id: ShortcutId,
    /// 按键组合
    pub combo: KeyCombo,
    /// 中文描述
    pub description_zh: &'static str,
    /// 英文描述
    pub description_en: &'static str,
}

impl Shortcut {
    /// 创建新的快捷键定义
    pub fn new(
        id: &'static str,
        combo: KeyCombo,
        description_zh: &'static str,
        description_en: &'static str,
    ) -> Self {
        Self {
            id: ShortcutId(id),
            combo,
            description_zh,
            description_en,
        }
    }
}
