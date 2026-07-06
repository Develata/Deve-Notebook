use super::super::collect_markdown_projection_files;
use std::fs;

#[test]
fn collect_markdown_projection_files_uploads_only_markdown_projection_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("notes")).expect("notes");
    fs::create_dir_all(dir.path().join(".notegit")).expect("notegit");
    fs::create_dir_all(dir.path().join(".git")).expect("git");
    fs::create_dir_all(dir.path().join("ledger")).expect("ledger");
    fs::write(dir.path().join("notes").join("a.md"), "a").expect("a");
    fs::write(dir.path().join("notes").join("b.markdown"), "b").expect("b");
    fs::write(dir.path().join("notes").join("skip.txt"), "skip").expect("txt");
    fs::write(dir.path().join(".notegit").join("secret.md"), "secret").expect("secret");
    fs::write(dir.path().join(".git").join("config.md"), "git").expect("git file");
    fs::write(dir.path().join("ledger").join("local.md"), "ledger").expect("ledger file");
    fs::write(dir.path().join(".deveignore"), "ignored.md\n").expect("ignore");
    fs::write(dir.path().join("ignored.md"), "ignored").expect("ignored");

    let files = collect_markdown_projection_files(dir.path()).expect("files");
    let paths = files
        .iter()
        .map(|file| file.path().to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["notes/a.md", "notes/b.markdown"]);
}

#[test]
fn collect_markdown_projection_files_skips_ignored_directories() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("ignored_dir")).expect("ignored dir");
    fs::create_dir_all(dir.path().join("kept_dir")).expect("kept dir");
    fs::write(dir.path().join(".deveignore"), "ignored_dir/\n").expect("ignore");
    fs::write(dir.path().join("ignored_dir").join("secret.md"), "secret").expect("secret");
    fs::write(dir.path().join("kept_dir").join("note.md"), "note").expect("note");

    let files = collect_markdown_projection_files(dir.path()).expect("files");
    let paths = files
        .iter()
        .map(|file| file.path().to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["kept_dir/note.md"]);
}
