#[cfg(test)]
mod tests {
    use deve_core::context::DirectoryTree;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_root() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("deve-context-tree-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("create src fixture");
        fs::create_dir_all(root.join(".git")).expect("create hidden git fixture");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n")
            .expect("write Cargo.toml fixture");
        fs::write(root.join("src").join("lib.rs"), "pub fn fixture() {}\n")
            .expect("write lib.rs fixture");
        fs::write(root.join(".git").join("config"), "[core]\n").expect("write hidden fixture");
        root
    }

    #[test]
    fn test_tree_generation() {
        let root = fixture_root();
        let tree = DirectoryTree::generate(&root);

        println!("Tree structure:\n{}", tree.structure);

        assert!(tree.structure.contains("Cargo.toml"));
        assert!(tree.structure.contains("src/"));
        assert!(tree.structure.contains("lib.rs"));

        assert!(!tree.structure.contains(".git/"));

        fs::remove_dir_all(root).expect("remove context tree fixture");
    }
}
