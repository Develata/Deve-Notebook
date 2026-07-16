//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!
pub fn diff_title(path: &str, display_path: &str) -> String {
    if display_path != path {
        return display_path.to_string();
    }

    path.replace('\\', "/")
        .split('/')
        .next_back()
        .unwrap_or("?")
        .to_string()
}
