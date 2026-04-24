//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Copied path mapping helpers.

use anyhow::{Result, anyhow};

pub(super) fn map_dest_rel(src_rel: &str, src_path: &str, dest_path: &str) -> Result<String> {
    let suffix = src_rel.strip_prefix(src_path).ok_or_else(|| {
        anyhow!(
            "Copied path {} is not under source root {}",
            src_rel,
            src_path
        )
    })?;
    let trimmed = suffix.trim_start_matches('/');
    if trimmed.is_empty() {
        return Ok(dest_path.to_string());
    }
    Ok(format!("{}/{}", dest_path.trim_end_matches('/'), trimmed))
}

#[cfg(test)]
mod tests {
    use super::map_dest_rel;

    #[test]
    fn map_dest_rel_fails_closed_when_source_rel_escapes_root() {
        let err = map_dest_rel("notes/b.md", "other", "dest")
            .expect_err("path outside source root must fail closed");
        assert!(err.to_string().contains("is not under source root"));
    }

    #[test]
    fn map_dest_rel_preserves_exact_root_copy() {
        assert_eq!(
            map_dest_rel("notes", "notes", "dest").expect("root copy maps"),
            "dest"
        );
    }
}
