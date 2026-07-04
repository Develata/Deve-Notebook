use super::*;
use crate::i18n::Locale;

#[test]
fn ngit_source_control_copy_is_localized() {
    assert_eq!(
        source_control_authority_title(Locale::En),
        "NoteGit/ngit authority; Git main is a terminal-state mirror"
    );
    assert_eq!(
        source_control_authority_badge(Locale::Zh, "ngit"),
        "ngit authority"
    );
    assert_eq!(
        git_status_cli_only_title(Locale::En),
        "ngit status is CLI-only"
    );
    assert!(git_status_cli_only_hint(Locale::Zh).contains("deve_cli ngit status"));
    assert_eq!(
        git_mirror_cli_only_title(Locale::Zh),
        "ngit mirror 执行只能通过 CLI 完成"
    );
    assert!(git_mirror_cli_only_hint(Locale::En).contains("deve_cli ngit mirror"));
    assert_eq!(
        git_export_cli_only_title(Locale::En),
        "ngit mirror export is CLI-only"
    );
    assert!(git_export_cli_only_hint(Locale::Zh).contains("deve_cli ngit export"));
    assert_eq!(
        git_import_cli_only_title(Locale::En),
        "ngit import is CLI-only"
    );
    assert!(git_import_cli_only_hint(Locale::En).contains("deve_cli ngit import --apply"));
    assert_eq!(
        git_push_cli_only_title(Locale::Zh),
        "ngit mirror 推送只能通过 CLI 执行"
    );
    assert!(git_push_cli_only_hint(Locale::Zh).contains("--remote <remote> --branch <branch>"));
    assert!(commit_and_push_cli_only_banner(Locale::Zh).contains("deve_cli ngit push"));
    assert!(
        git_push_cli_only_details(Locale::En)
            .iter()
            .any(|line| line.contains("deve_cli ngit export --repo <repo>"))
    );
    assert!(
        git_push_cli_only_details(Locale::Zh)
            .iter()
            .any(|line| line.contains("deve_cli ngit import --apply --repo <repo>"))
    );
    assert_eq!(
        git_repair_cli_only_title(Locale::En),
        "ngit mirror repair is CLI-only"
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
        "deve_cli ngit export --repo <repo> --retry-out-of-sync"
    );
    assert!(git_repair_authority_note(Locale::En).contains("read-only"));
    assert_eq!(
        git_import_conflict_title(Locale::Zh),
        "导入冲突：暂存前请选择保留文件系统版本或账本版本。"
    );
}

#[test]
fn source_control_header_source_control_authority_badge() {
    assert_eq!(
        source_control_authority_badge(Locale::En, "ngit"),
        "ngit authority"
    );
    assert_eq!(
        source_control_authority_badge(Locale::Zh, "unexpected"),
        "ngit unknown"
    );
    assert_eq!(
        source_control_authority_badge(Locale::En, "unknown"),
        "ngit unknown"
    );
}
