//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
use crate::i18n::{Locale, t};
use deve_core::models::PeerId;

pub fn current_branch_label(active_branch: Option<PeerId>, locale: Locale) -> String {
    active_branch
        .map(|peer| peer.to_string())
        .unwrap_or_else(|| t::sidebar::local_branch(locale).to_string())
}

#[cfg(test)]
mod tests {
    use super::current_branch_label;
    use crate::i18n::Locale;
    use deve_core::models::PeerId;

    #[test]
    fn uses_local_label_when_no_remote_branch_is_active() {
        assert_eq!(current_branch_label(None, Locale::En), "Local");
    }

    #[test]
    fn uses_peer_id_for_remote_branch() {
        assert_eq!(
            current_branch_label(Some(PeerId::new("peer-a")), Locale::En),
            "peer-a"
        );
    }
}
