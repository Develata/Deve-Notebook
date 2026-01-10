//! Cross-platform path utilities
//!
//! This module provides utilities for handling file paths across different
//! operating systems (Windows, Linux, macOS).

use std::path::PathBuf;

/// Normalize a path string to use the system's native path separator.
///
/// # Behavior
/// - On Windows: `foo/bar` becomes `foo\bar`
/// - On Linux/macOS: `foo\bar` becomes `foo/bar`
///
/// # Example
/// ```
/// use deve_core::utils::path::normalize;
/// let normalized = normalize("folder/subfolder/file.md");
/// ```
pub fn normalize(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        path.replace('/', "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.replace('\\', "/")
    }
}

/// Join a base path with a normalized subpath.
///
/// This function first normalizes the subpath to use the system's native
/// path separator, then joins it with the base path.
///
/// # Example
/// ```
/// use std::path::PathBuf;
/// use deve_core::utils::path::join_normalized;
/// let base = PathBuf::from("/vault");
/// let result = join_normalized(&base, "folder/file.md");
/// ```
pub fn join_normalized(base: &PathBuf, subpath: &str) -> PathBuf {
    base.join(normalize(subpath))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_forward_slashes() {
        let input = "folder/subfolder/file.md";
        let result = normalize(input);
        
        #[cfg(target_os = "windows")]
        assert_eq!(result, "folder\\subfolder\\file.md");
        
        #[cfg(not(target_os = "windows"))]
        assert_eq!(result, "folder/subfolder/file.md");
    }

    #[test]
    fn test_normalize_backslashes() {
        let input = "folder\\subfolder\\file.md";
        let result = normalize(input);
        
        #[cfg(target_os = "windows")]
        assert_eq!(result, "folder\\subfolder\\file.md");
        
        #[cfg(not(target_os = "windows"))]
        assert_eq!(result, "folder/subfolder/file.md");
    }

    #[test]
    fn test_join_normalized() {
        let base = PathBuf::from("vault");
        let result = join_normalized(&base, "folder/file.md");
        
        #[cfg(target_os = "windows")]
        assert_eq!(result, PathBuf::from("vault\\folder\\file.md"));
        
        #[cfg(not(target_os = "windows"))]
        assert_eq!(result, PathBuf::from("vault/folder/file.md"));
    }
}
