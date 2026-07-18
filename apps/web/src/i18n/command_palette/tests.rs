use super::*;
use crate::i18n::Locale;

#[test]
fn ngit_and_remote_projection_commands_are_localized() {
    assert_eq!(
        establish_branch_unavailable_reason(Locale::Zh),
        "不可用：尚无分支创建后端"
    );
    assert_eq!(source_control_commit(Locale::En), "Source Control: Commit");
    assert_eq!(group_source_control(Locale::Zh), "源代码管理");
    assert!(source_control_panel_reason(Locale::En).contains("Source Control panel"));
    assert!(source_control_panel_reason(Locale::Zh).contains("源代码管理面板"));
    assert_eq!(ngit_status(Locale::En), "ngit: status");
    assert_eq!(ngit_mirror(Locale::Zh), "ngit: 执行 mirror");
    assert_eq!(ngit_export_mirror(Locale::En), "ngit: export mirror");
    assert!(ngit_cli_only_reason(Locale::Zh).contains("CLI-only"));
    assert_eq!(ngit_import_changes(Locale::En), "ngit: import changes");
    assert_eq!(ngit_import_changes(Locale::Zh), "ngit: 导入外部变更");
    assert_eq!(ngit_push_mirror(Locale::En), "ngit: push mirror");
    assert_eq!(ngit_push_mirror(Locale::Zh), "ngit: 推送 mirror");
    assert_eq!(ngit_repair_mirror(Locale::En), "ngit: repair mirror");
    assert_eq!(ngit_repair_mirror(Locale::Zh), "ngit: 修复 mirror");
    assert_eq!(webdav_push(Locale::En), "Remote Projection: WebDAV Push");
    assert_eq!(s3_push(Locale::Zh), "远程投影：S3 推送");
    assert_eq!(
        remote_projection_scope_unavailable(Locale::En),
        "Unavailable: current repository scope is not ready"
    );
    assert_eq!(
        remote_projection_scope_unavailable(Locale::Zh),
        "不可用：当前仓库作用域尚未就绪"
    );
    assert_eq!(toggle_sidebar(Locale::En), "Toggle Sidebar");
    assert_eq!(ai_switch_plan(Locale::En), "AI: Switch to PLAN Mode");
    assert!(ai_slash_mode_reason(Locale::En).contains("/plan"));
}
