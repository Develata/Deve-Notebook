use super::*;

#[test]
fn git_bridge_commands_are_localized() {
    assert_eq!(
        establish_branch_unavailable_reason(Locale::Zh),
        "不可用：尚无分支创建后端"
    );
    assert_eq!(source_control_commit(Locale::En), "Source Control: Commit");
    assert!(source_control_panel_reason(Locale::En).contains("Source Control panel"));
    assert_eq!(git_status(Locale::En), "Git: Status");
    assert_eq!(git_mirror(Locale::Zh), "Git: 执行 Mirror");
    assert_eq!(git_export_mirror(Locale::En), "Git: Export Mirror");
    assert!(git_cli_only_reason(Locale::Zh).contains("CLI-only"));
    assert_eq!(git_import_changes(Locale::En), "Git: Import Changes");
    assert_eq!(git_import_changes(Locale::Zh), "Git: 导入外部变更");
    assert_eq!(git_push_mirror(Locale::En), "Git: Push Mirror");
    assert_eq!(git_push_mirror(Locale::Zh), "Git: 推送 Mirror");
    assert_eq!(git_repair_mirror(Locale::En), "Git: Repair Mirror");
    assert_eq!(git_repair_mirror(Locale::Zh), "Git: 修复 Mirror");
    assert_eq!(toggle_sidebar(Locale::En), "Toggle Sidebar");
    assert_eq!(ai_switch_plan(Locale::En), "AI: Switch to PLAN Mode");
    assert!(ai_slash_mode_reason(Locale::En).contains("/plan"));
}
