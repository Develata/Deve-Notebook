//! plan_ref:
//!   - 11_ui_design_01_web#single-binary-distribution
//!
//! Build-time frontend asset embedding.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=DEVE_EMBED_STATIC_DIR");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let dist_dir = env::var_os("DEVE_EMBED_STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../web/dist"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("out dir")).join("embedded_static.rs");

    let code = embedded_static_source(&dist_dir);
    fs::write(out, code).expect("write embedded_static.rs");
}

fn embedded_static_source(dist_dir: &Path) -> String {
    track_dist_inputs(dist_dir);
    let mut files = Vec::new();
    collect_files(dist_dir, dist_dir, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut code = String::from(
        "pub(crate) struct EmbeddedAsset {\n\
         pub(crate) path: &'static str,\n\
         pub(crate) bytes: &'static [u8],\n\
         }\n\
         pub(crate) static EMBEDDED_ASSETS: &[EmbeddedAsset] = &[\n",
    );
    for (rel, abs) in files {
        code.push_str(&format!(
            "EmbeddedAsset {{ path: {rel:?}, bytes: include_bytes!({abs:?}) }},\n",
            abs = abs.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    code
}

fn track_dist_inputs(dist_dir: &Path) {
    println!("cargo:rerun-if-changed={}", dist_dir.display());
    if let Some(parent) = dist_dir.parent() {
        println!("cargo:rerun-if-changed={}", parent.display());
    }
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else if path.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            files.push((to_forward_slash(rel), path));
        }
    }
}

fn to_forward_slash(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
