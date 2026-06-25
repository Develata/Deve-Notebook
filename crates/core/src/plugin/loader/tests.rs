use super::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_plugin_loader() {
    // 1. Setup temp plugin directory
    let dir = tempdir().unwrap();
    let plugin_dir = dir.path().join("my-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // 2. Create manifest.json
    let manifest_content = r#"{
        "id": "test-plugin",
        "name": "Test Plugin",
        "version": "1.0.0",
        "entry": "index.rhai",
        "capabilities": {
            "allow_env": ["USER"]
        }
    }"#;
    fs::write(plugin_dir.join("manifest.json"), manifest_content).unwrap();

    // 3. Create entry script
    let script_content = r#"
        fn hello() {
            return "world";
        }
    "#;
    fs::write(plugin_dir.join("index.rhai"), script_content).unwrap();

    // 4. Load
    let loader = PluginLoader::new(dir.path().to_path_buf());
    let plugins = loader.load_all().expect("Failed to load plugins");

    assert_eq!(plugins.len(), 1);
    let plugin = &plugins[0];
    assert_eq!(plugin.manifest().id, "test-plugin");

    let res = plugin.call("hello", vec![]).expect("Failed to call");
    assert_eq!(res.clone().into_string().unwrap(), "world");
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), unix))]
fn load_all_fails_closed_when_plugin_dir_is_unstatable() {
    let dir = tempdir().expect("tempdir");
    let original = fs::metadata(dir.path()).expect("metadata").permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    fs::set_permissions(dir.path(), blocked).expect("chmod 000");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_all() {
        Ok(_) => panic!("unstatable plugin dir must fail closed"),
        Err(err) => err,
    };

    fs::set_permissions(dir.path(), original).expect("restore perms");
    assert!(
        err.to_string().contains("Failed to stat plugin directory")
            || err.to_string().contains("Permission denied")
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_all_strict_fails_closed_when_any_plugin_is_broken() {
    let dir = tempdir().expect("tempdir");

    let good = dir.path().join("good-plugin");
    fs::create_dir(&good).expect("mkdir good");
    fs::write(
        good.join("manifest.json"),
        r#"{
            "id": "good-plugin",
            "name": "Good Plugin",
            "version": "1.0.0",
            "entry": "index.rhai"
        }"#,
    )
    .expect("write good manifest");
    fs::write(good.join("index.rhai"), "fn hello() { \"ok\" }").expect("write good entry");

    let broken = dir.path().join("broken-plugin");
    fs::create_dir(&broken).expect("mkdir broken");
    fs::write(
        broken.join("manifest.json"),
        r#"{
            "id": "broken-plugin",
            "name": "Broken Plugin",
            "version": "1.0.0",
            "entry": "missing.rhai"
        }"#,
    )
    .expect("write broken manifest");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_all_strict() {
        Ok(_) => panic!("broken plugin must fail closed in strict mode"),
        Err(err) => err,
    };

    assert!(
        err.to_string().contains("Failed to load plugin at")
            || err.to_string().contains("Missing entry script")
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_plugin_rejects_entry_parent_traversal() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": "../outside.rhai"
        }"#,
    )
    .expect("write manifest");
    fs::write(dir.path().join("outside.rhai"), "fn hello() { \"bad\" }").expect("write outside");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("entry traversal must be rejected"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("Invalid plugin entry"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_plugin_rejects_absolute_entry_path() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    let outside = dir
        .path()
        .join("outside.rhai")
        .to_string_lossy()
        .replace('\\', "/");
    let manifest_content = serde_json::json!({
        "id": "bad-plugin",
        "name": "Bad Plugin",
        "version": "1.0.0",
        "entry": outside
    })
    .to_string();
    fs::write(plugin_dir.join("manifest.json"), manifest_content).expect("write manifest");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("absolute entry path must be rejected"),
        Err(err) => err,
    };

    let err = err.to_string();
    assert!(
        err.contains("absolute paths are not allowed")
            || err.contains("drive prefixes are not allowed")
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_plugin_rejects_windows_drive_prefix_entry_path() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": "C:/outside.rhai"
        }"#,
    )
    .expect("write manifest");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("drive-prefixed entry path must be rejected"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("drive prefixes are not allowed"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_plugin_rejects_backslash_entry_path() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": "scripts\\index.rhai"
        }"#,
    )
    .expect("write manifest");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("backslash entry path must be rejected"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("forward-slash relative paths"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_plugin_rejects_non_rhai_entry() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": "index.txt"
        }"#,
    )
    .expect("write manifest");
    fs::write(plugin_dir.join("index.txt"), "fn hello() { \"bad\" }").expect("write entry");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("non-rhai entry must be rejected"),
        Err(err) => err,
    };

    assert!(err.to_string().contains(".rhai"));
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), unix))]
fn load_plugin_rejects_symlinked_entry_outside_plugin_dir() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": "index.rhai"
        }"#,
    )
    .expect("write manifest");
    let outside = dir.path().join("outside.rhai");
    fs::write(&outside, "fn hello() { \"bad\" }").expect("write outside");
    std::os::unix::fs::symlink(&outside, plugin_dir.join("index.rhai")).expect("symlink entry");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("symlinked outside entry must be rejected"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("resolved entry must stay inside plugin directory")
    );
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), unix))]
fn load_plugin_rejects_entry_through_symlinked_directory() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("bad-plugin");
    fs::create_dir(&plugin_dir).expect("mkdir plugin");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": "scripts/index.rhai"
        }"#,
    )
    .expect("write manifest");
    let outside_dir = dir.path().join("outside-scripts");
    fs::create_dir(&outside_dir).expect("mkdir outside scripts");
    fs::write(outside_dir.join("index.rhai"), "fn hello() { \"bad\" }")
        .expect("write outside entry");
    std::os::unix::fs::symlink(&outside_dir, plugin_dir.join("scripts")).expect("symlink scripts");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let err = match loader.load_plugin(&plugin_dir) {
        Ok(_) => panic!("entry through symlinked outside directory must be rejected"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("resolved entry must stay inside plugin directory")
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_plugin_allows_nested_rhai_entry() {
    let dir = tempdir().expect("tempdir");
    let plugin_dir = dir.path().join("nested-plugin");
    let script_dir = plugin_dir.join("scripts");
    fs::create_dir_all(&script_dir).expect("mkdir scripts");
    fs::write(
        plugin_dir.join("manifest.json"),
        r#"{
            "id": "nested-plugin",
            "name": "Nested Plugin",
            "version": "1.0.0",
            "entry": "scripts/index.rhai"
        }"#,
    )
    .expect("write manifest");
    fs::write(script_dir.join("index.rhai"), "fn hello() { \"ok\" }").expect("write entry");

    let loader = PluginLoader::new(dir.path().to_path_buf());
    let plugin = loader.load_plugin(&plugin_dir).expect("load plugin");

    assert_eq!(plugin.manifest().id, "nested-plugin");
}
