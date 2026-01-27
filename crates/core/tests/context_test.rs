#[cfg(test)]
mod tests {
    use deve_core::context::DirectoryTree;
    use std::path::Path;

    #[test]
    fn test_tree_generation() {
        // Use the current project root as test case
        let root = Path::new(".");
        let tree = DirectoryTree::generate(root);

        println!("Tree structure:\n{}", tree.structure);

        // Assertions adjusted for crate-level execution
        assert!(tree.structure.contains("Cargo.toml"));
        assert!(tree.structure.contains("src/")); // Should contain src directory
        assert!(tree.structure.contains("lib.rs")); // Should contain lib.rs inside src

        // Ensure .git is ignored
        assert!(!tree.structure.contains(".git/"));
    }
}
