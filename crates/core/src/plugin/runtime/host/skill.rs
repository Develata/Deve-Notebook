// crates/core/src/plugin/runtime/host/skill.rs
//! # Skill 宿主函数
//!
//! **功能**: 暴露 SkillManager 给 Rhai 脚本使用。
//! **说明**: 技能文件为 Markdown，按需加载以节省 Tokens。

use crate::plugin::manifest::Capability;
use crate::skill::SkillManager;
use anyhow::Result;
use rhai::{Engine, EvalAltResult};
use std::path::PathBuf;
use std::sync::Arc;

fn skill_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from(".deve/skills"),
        PathBuf::from(".opencode/skill"),
        PathBuf::from(".opencode/skills"),
        PathBuf::from(".claude/skills"),
    ]
}

fn list_all_skills_in(dirs: &[PathBuf]) -> Result<Vec<crate::skill::Skill>> {
    let mut all = Vec::new();
    for dir in dirs {
        let manager = SkillManager::new(dir.clone());
        all.extend(manager.list()?);
    }
    Ok(all)
}

fn list_all_skills() -> Result<Vec<crate::skill::Skill>> {
    list_all_skills_in(&skill_dirs())
}

fn load_skill_by_name_in(name: &str, dirs: &[PathBuf]) -> Result<Option<crate::skill::Skill>> {
    for dir in dirs {
        let manager = SkillManager::new(dir.clone());
        if let Some(skill) = manager.get(name)? {
            return Ok(Some(skill));
        }
    }
    Ok(None)
}

fn load_skill_by_name(name: &str) -> Result<Option<crate::skill::Skill>> {
    load_skill_by_name_in(name, &skill_dirs())
}

/// 注册 Skill API
pub fn register_skill_api(engine: &mut Engine, caps: Arc<Capability>) {
    // API: list_skills() -> Array<Map>
    let caps_list = caps.clone();
    engine.register_fn(
        "list_skills",
        move || -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_list.check_skill() {
                return Err("Permission denied: skill access not allowed by manifest.".into());
            }
            let skills = list_all_skills().map_err(|e| e.to_string())?;
            let items: Vec<serde_json::Value> = skills
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "description": s.description,
                    })
                })
                .collect();
            let json = serde_json::Value::Array(items);
            rhai::serde::to_dynamic(&json).map_err(|e| e.to_string().into())
        },
    );

    // API: get_skill(name: &str) -> Map | ()
    let caps_get = caps.clone();
    engine.register_fn(
        "get_skill",
        move |name: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            if !caps_get.check_skill() {
                return Err("Permission denied: skill access not allowed by manifest.".into());
            }
            match load_skill_by_name(name).map_err(|e| e.to_string())? {
                Some(skill) => {
                    let json = serde_json::json!({
                        "name": skill.name,
                        "description": skill.description,
                        "content": skill.content,
                    });
                    rhai::serde::to_dynamic(&json).map_err(|e| e.to_string().into())
                }
                None => Ok(rhai::Dynamic::UNIT),
            }
        },
    );
}

#[cfg(test)]
#[path = "skill_test.rs"]
mod tests;
