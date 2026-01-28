// crates/core/src/plugin/runtime/host/skill.rs
//! # Skill 宿主函数
//!
//! **功能**: 暴露 SkillManager 给 Rhai 脚本使用。
//! **说明**: 技能文件为 Markdown，按需加载以节省 Tokens。

use crate::skill::SkillManager;
use rhai::{Engine, EvalAltResult};
use std::path::PathBuf;

fn skill_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from(".deve/skills"),
        PathBuf::from(".opencode/skill"),
        PathBuf::from(".opencode/skills"),
        PathBuf::from(".claude/skills"),
    ]
}

fn list_all_skills() -> Vec<crate::skill::Skill> {
    let mut all = Vec::new();
    for dir in skill_dirs() {
        let manager = SkillManager::new(dir);
        if let Ok(skills) = manager.list() {
            all.extend(skills);
        }
    }
    all
}

fn load_skill_by_name(name: &str) -> Option<crate::skill::Skill> {
    for dir in skill_dirs() {
        let manager = SkillManager::new(dir);
        if let Ok(Some(skill)) = manager.get(name) {
            return Some(skill);
        }
    }
    None
}

/// 注册 Skill API
pub fn register_skill_api(engine: &mut Engine) {
    // API: list_skills() -> Array<Map>
    engine.register_fn(
        "list_skills",
        move || -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            let skills = list_all_skills();
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
    engine.register_fn(
        "get_skill",
        move |name: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
            match load_skill_by_name(name) {
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
