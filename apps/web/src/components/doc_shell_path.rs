//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! Shared path policy for Web shell-like document commands.

pub(crate) fn is_doc_shell_path_representable(path: &str) -> bool {
    !path.chars().any(is_doc_shell_reserved_char)
}

fn is_doc_shell_reserved_char(ch: char) -> bool {
    ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_shell_path_allows_portable_markdown_paths() {
        assert!(is_doc_shell_path_representable("notes/today.md"));
        assert!(is_doc_shell_path_representable("space dir/today note.md"));
    }

    #[test]
    fn doc_shell_path_rejects_reserved_command_chars() {
        for ch in ['<', '>', ':', '"', '|', '?', '*', '\n'] {
            let path = format!("notes/a{ch}b.md");
            assert!(!is_doc_shell_path_representable(&path));
        }
    }
}
