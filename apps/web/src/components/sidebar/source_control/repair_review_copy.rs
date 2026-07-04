//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Read-only Git mirror repair review copy model.

use crate::api::GitMirrorRepairReview;
use crate::i18n::{Locale, source_control as sc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitRepairReviewFetchState {
    Idle,
    Loading,
    Loaded(GitMirrorRepairReview),
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepairReviewCopy {
    pub title: String,
    pub status_note: Option<String>,
    pub records: Vec<GitRepairReviewRecordCopy>,
    pub authority_note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepairReviewRecordCopy {
    pub heading: String,
    pub rows: Vec<GitRepairReviewRow>,
    pub retry_command: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepairReviewRow {
    pub label: String,
    pub value: String,
}

pub fn git_repair_review(locale: Locale, state: &GitRepairReviewFetchState) -> GitRepairReviewCopy {
    match state {
        GitRepairReviewFetchState::Loading => static_git_repair_review(
            locale,
            Some(sc::git_repair_review_loading(locale).to_string()),
        ),
        GitRepairReviewFetchState::Loaded(review) if !review.records.is_empty() => {
            git_repair_review_from_data(locale, review)
        }
        GitRepairReviewFetchState::Loaded(_) => static_git_repair_review(
            locale,
            Some(sc::git_repair_review_empty(locale).to_string()),
        ),
        GitRepairReviewFetchState::Failed => static_git_repair_review(
            locale,
            Some(sc::git_repair_review_load_failed(locale).to_string()),
        ),
        GitRepairReviewFetchState::Idle => static_git_repair_review(locale, None),
    }
}

fn static_git_repair_review(locale: Locale, status_note: Option<String>) -> GitRepairReviewCopy {
    GitRepairReviewCopy {
        title: sc::git_repair_review_title(locale).to_string(),
        status_note,
        records: vec![GitRepairReviewRecordCopy {
            heading: sc::git_repair_review_fallback_record(locale).to_string(),
            rows: vec![
                GitRepairReviewRow {
                    label: sc::git_repair_action_label(locale).to_string(),
                    value: sc::git_repair_action_value(locale).to_string(),
                },
                GitRepairReviewRow {
                    label: sc::git_repair_guidance_label(locale).to_string(),
                    value: sc::git_repair_guidance_value(locale).to_string(),
                },
                GitRepairReviewRow {
                    label: sc::git_repair_subject_label(locale).to_string(),
                    value: sc::git_repair_subject_value(locale).to_string(),
                },
                GitRepairReviewRow {
                    label: sc::git_repair_next_step_label(locale).to_string(),
                    value: sc::git_repair_next_step_value(locale).to_string(),
                },
            ],
            retry_command: sc::git_repair_retry_command().to_string(),
        }],
        authority_note: sc::git_repair_authority_note(locale).to_string(),
    }
}

fn git_repair_review_from_data(
    locale: Locale,
    review: &GitMirrorRepairReview,
) -> GitRepairReviewCopy {
    GitRepairReviewCopy {
        title: sc::git_repair_review_title(locale).to_string(),
        status_note: Some(sc::git_repair_review_loaded_count(
            locale,
            review.records.len(),
        )),
        records: review
            .records
            .iter()
            .map(|record| GitRepairReviewRecordCopy {
                heading: format!("{} #{}", record.action_code, record.ledger_seq),
                rows: vec![
                    GitRepairReviewRow {
                        label: sc::git_repair_commit_label(locale).to_string(),
                        value: record.deve_commit_id.clone(),
                    },
                    GitRepairReviewRow {
                        label: sc::git_repair_subject_label(locale).to_string(),
                        value: record.subject.clone(),
                    },
                    GitRepairReviewRow {
                        label: sc::git_repair_next_step_label(locale).to_string(),
                        value: record.next_step.clone(),
                    },
                    GitRepairReviewRow {
                        label: sc::git_repair_guidance_label(locale).to_string(),
                        value: "manual_only=yes".to_string(),
                    },
                ],
                retry_command: record
                    .retry_command
                    .clone()
                    .unwrap_or_else(|| sc::git_repair_retry_command().to_string()),
            })
            .collect(),
        authority_note: review.authority.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{GitRepairReviewFetchState, git_repair_review};
    use crate::api::{GitMirrorRepairReview, GitMirrorRepairReviewRecord};
    use crate::i18n::Locale;

    #[test]
    fn repair_review_idle_uses_static_cli_fallback() {
        let review = git_repair_review(Locale::En, &GitRepairReviewFetchState::Idle);

        assert_eq!(review.records.len(), 1);
        assert!(
            review.records[0]
                .rows
                .iter()
                .any(|row| row.value.contains("manual_only=yes"))
        );
        assert_eq!(
            review.records[0].retry_command,
            "deve_cli ngit export --repo <repo> --retry-out-of-sync"
        );
    }

    #[test]
    fn repair_review_loading_and_error_keep_static_fallback() {
        let loading = git_repair_review(Locale::En, &GitRepairReviewFetchState::Loading);
        let failed = git_repair_review(Locale::En, &GitRepairReviewFetchState::Failed);

        assert!(loading.status_note.unwrap().contains("Loading"));
        assert!(failed.status_note.unwrap().contains("could not load"));
        assert_eq!(
            loading.records[0].retry_command,
            failed.records[0].retry_command
        );
    }

    #[test]
    fn repair_review_renders_multiple_record_level_rows() {
        let data = GitMirrorRepairReview {
            repo_name: "default".to_string(),
            manual_only: true,
            authority: "server-side authority note".to_string(),
            records: vec![record("deve-1", 1), record("deve-2", 2)],
        };

        let review = git_repair_review(Locale::En, &GitRepairReviewFetchState::Loaded(data));

        assert_eq!(review.records.len(), 2);
        assert!(review.status_note.unwrap().contains("2"));
        assert_eq!(review.records[0].heading, "resolve_projection_scope #1");
        assert!(
            review.records[1]
                .rows
                .iter()
                .any(|row| row.value == "docs/deve-2.md")
        );
        assert_eq!(review.authority_note, "server-side authority note");
    }

    #[test]
    fn repair_review_loaded_empty_reports_empty_fallback() {
        let data = GitMirrorRepairReview {
            repo_name: "default".to_string(),
            manual_only: true,
            authority: "server-side authority note".to_string(),
            records: Vec::new(),
        };

        let review = git_repair_review(Locale::En, &GitRepairReviewFetchState::Loaded(data));

        assert_eq!(review.records.len(), 1);
        assert!(review.status_note.unwrap().contains("No out-of-sync"));
    }

    fn record(deve_commit_id: &str, ledger_seq: u64) -> GitMirrorRepairReviewRecord {
        GitMirrorRepairReviewRecord {
            deve_commit_id: deve_commit_id.to_string(),
            ledger_seq,
            action_code: "resolve_projection_scope".to_string(),
            retryable_after_fix: true,
            subject: format!("docs/{deve_commit_id}.md"),
            next_step: "fix_projection_or_path_subject".to_string(),
            retry_command: Some(
                "deve_cli ngit export --repo default --retry-out-of-sync".to_string(),
            ),
            failure_stage: Some("projection_scope".to_string()),
            failure_command: None,
            failure_exit_status: None,
            last_error: Some("failure".to_string()),
        }
    }
}
