use super::*;
use crate::i18n::Locale;

#[test]
fn git_bridge_source_control_copy_is_localized() {
    assert_eq!(
        git_bridge_mode_title(Locale::En),
        "NoteGit authority; Git is an optional bridge"
    );
    assert_eq!(
        git_bridge_mode_badge(Locale::Zh, "mirror"),
        "NoteGit + Git mirror"
    );
    assert_eq!(
        git_status_cli_only_title(Locale::En),
        "Git status is CLI-only"
    );
    assert!(git_status_cli_only_hint(Locale::Zh).contains("deve_cli git status"));
    assert_eq!(
        git_mirror_cli_only_title(Locale::Zh),
        "Git mirror 执行只能通过 CLI 完成"
    );
    assert!(git_mirror_cli_only_hint(Locale::En).contains("deve_cli git mirror"));
    assert_eq!(
        git_export_cli_only_title(Locale::En),
        "Git mirror export is CLI-only"
    );
    assert!(git_export_cli_only_hint(Locale::Zh).contains("deve_cli git export"));
    assert_eq!(
        git_import_cli_only_title(Locale::En),
        "Git import is CLI-only"
    );
    assert!(git_import_cli_only_hint(Locale::En).contains("deve_cli git import --apply"));
    assert_eq!(
        git_push_cli_only_title(Locale::Zh),
        "Git mirror 推送只能通过 CLI 执行"
    );
    assert!(git_push_cli_only_hint(Locale::Zh).contains("--remote <remote> --branch <branch>"));
    assert!(commit_and_push_cli_only_banner(Locale::Zh).contains("deve_cli git push"));
    assert!(
        git_push_cli_only_details(Locale::En)
            .iter()
            .any(|line| line.contains("deve_cli git export --repo <repo>"))
    );
    assert!(
        git_push_cli_only_details(Locale::Zh)
            .iter()
            .any(|line| line.contains("deve_cli git import --apply --repo <repo>"))
    );
    assert_eq!(
        git_repair_cli_only_title(Locale::En),
        "Git mirror repair is CLI-only"
    );
    assert!(git_repair_cli_only_hint(Locale::Zh).contains("repair_action[...]"));
    assert!(
        git_repair_cli_only_details(Locale::En)
            .iter()
            .any(|line| line.contains("retry-out-of-sync"))
    );
    assert_eq!(git_repair_review_title(Locale::Zh), "只读修复审阅");
    assert!(git_repair_review_loading(Locale::En).contains("Loading"));
    assert!(git_repair_review_load_failed(Locale::En).contains("could not load"));
    assert!(git_repair_review_empty(Locale::En).contains("No out-of-sync"));
    assert!(git_repair_review_loaded_count(Locale::En, 2).contains("2"));
    assert_eq!(
        git_repair_review_fallback_record(Locale::Zh),
        "CLI fallback"
    );
    assert!(git_repair_guidance_value(Locale::En).contains("manual_only=yes"));
    assert_eq!(git_repair_commit_label(Locale::En), "Commit");
    assert_eq!(
        git_repair_retry_command(),
        "deve_cli git export --repo <repo> --retry-out-of-sync"
    );
    assert!(git_repair_authority_note(Locale::En).contains("read-only"));
    assert_eq!(
        git_import_conflict_title(Locale::Zh),
        "导入冲突：暂存前请选择保留文件系统版本或账本版本。"
    );
}

#[test]
fn source_control_header_git_bridge_mode_badge() {
    assert_eq!(git_bridge_mode_badge(Locale::En, "off"), "NoteGit only");
    assert_eq!(
        git_bridge_mode_badge(Locale::Zh, "mirror"),
        "NoteGit + Git mirror"
    );
    assert_eq!(
        git_bridge_mode_badge(Locale::En, "native"),
        "NoteGit + Git unknown"
    );
}
