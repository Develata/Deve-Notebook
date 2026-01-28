// plugins/ai-chat/commands.rhai

fn handle_command(user_message) {
    if user_message == "/plan" {
        state::set_mode(state::mode_plan());
        return "已切换到 PLAN 模式。请描述你的目标，我会输出详细计划。";
    }
    if user_message == "/build" {
        state::set_mode(state::mode_build());
        return "已切换到 BUILD 模式。请告诉我需要实现的功能或修复。";
    }
    if user_message == "/skills" {
        let skills_text = format::format_skills();
        return `可用技能:\n${skills_text}`;
    }
    if user_message.starts_with("/skill ") {
        let name = user_message.replace("/skill ", "").trim();
        if name == "" { return "请提供技能名称，例如 /skill math-teacher"; }
        let skill = get_skill(name);
        if skill == () { return "未找到该技能"; }
        state::add_active_skill(name);
        return `已加载技能: ${name}`;
    }
    return ();
}
