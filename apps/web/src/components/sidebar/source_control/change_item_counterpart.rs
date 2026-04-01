use crate::i18n::Locale;
use deve_core::source_control::ChangeEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterpartKind {
    Staged,
    WorkingTree,
}

pub fn find_counterpart_kind(
    entry: &ChangeEntry,
    is_staged: bool,
    staged: &[ChangeEntry],
    unstaged: &[ChangeEntry],
) -> Option<CounterpartKind> {
    let counterpart_entries = if is_staged { unstaged } else { staged };
    counterpart_entries
        .iter()
        .any(|other| same_visible_change(entry, other))
        .then_some(if is_staged {
            CounterpartKind::WorkingTree
        } else {
            CounterpartKind::Staged
        })
}

pub fn counterpart_badge_text(kind: CounterpartKind, locale: Locale) -> &'static str {
    match (kind, locale) {
        (CounterpartKind::Staged, Locale::En) => "IDX",
        (CounterpartKind::Staged, Locale::Zh) => "暂存区",
        (CounterpartKind::WorkingTree, Locale::En) => "WT",
        (CounterpartKind::WorkingTree, Locale::Zh) => "工作区",
    }
}

pub fn counterpart_badge_title(kind: CounterpartKind, locale: Locale) -> &'static str {
    match (kind, locale) {
        (CounterpartKind::Staged, Locale::En) => "Also present in Staged Changes",
        (CounterpartKind::Staged, Locale::Zh) => "对应改动也存在于暂存区",
        (CounterpartKind::WorkingTree, Locale::En) => "Also modified in Working Directory",
        (CounterpartKind::WorkingTree, Locale::Zh) => "对应改动也存在于工作区",
    }
}

fn same_visible_change(left: &ChangeEntry, right: &ChangeEntry) -> bool {
    left.path == right.path
        && left.doc_id == right.doc_id
        && left.status == right.status
        && left.renamed_from == right.renamed_from
}

#[cfg(test)]
mod tests {
    use super::{CounterpartKind, find_counterpart_kind};
    use deve_core::models::DocId;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    fn entry(path: &str, doc_id: Option<DocId>) -> ChangeEntry {
        ChangeEntry {
            path: path.into(),
            renamed_from: None,
            doc_id,
            status: ChangeStatus::Modified,
            has_conflict: false,
        }
    }

    #[test]
    fn finds_working_tree_counterpart_for_staged_entry() {
        let doc_id = DocId::new();
        assert_eq!(
            find_counterpart_kind(
                &entry("note.md", Some(doc_id)),
                true,
                &[],
                &[entry("note.md", Some(doc_id))]
            ),
            Some(CounterpartKind::WorkingTree)
        );
    }

    #[test]
    fn ignores_same_path_with_different_doc_id() {
        assert_eq!(
            find_counterpart_kind(
                &entry("note.md", Some(DocId::new())),
                true,
                &[],
                &[entry("note.md", Some(DocId::new()))]
            ),
            None
        );
    }
}
