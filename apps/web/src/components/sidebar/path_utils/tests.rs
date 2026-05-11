use super::*;
use uuid::Uuid;

fn make_docs(paths: &[&str]) -> Vec<(DocId, String)> {
    paths
        .iter()
        .enumerate()
        .map(|(i, p)| (DocId(Uuid::from_u128(i as u128)), p.to_string()))
        .collect()
}

#[test]
fn test_no_conflict() {
    let docs = make_docs(&["a.md", "b.md"]);
    assert_eq!(find_available_path("c.md", &docs), "c.md");
}

#[test]
fn test_first_copy() {
    let docs = make_docs(&["note.md"]);
    assert_eq!(find_available_path("note.md", &docs), "note copy.md");
}

#[test]
fn test_numbered_copy() {
    let docs = make_docs(&["note.md", "note copy.md"]);
    assert_eq!(find_available_path("note.md", &docs), "note copy 2.md");
}

#[test]
fn test_folder_copy() {
    let _docs = make_docs(&["folder/a.md"]);
    let folder_docs: Vec<(DocId, String)> = vec![];
    assert_eq!(find_available_path("folder", &folder_docs), "folder");
}

#[test]
fn test_split_ext() {
    assert_eq!(split_path_ext("note.md"), ("note", ".md"));
    assert_eq!(split_path_ext("notes/daily.md"), ("notes/daily", ".md"));
    assert_eq!(split_path_ext("folder"), ("folder", ""));
    assert_eq!(split_path_ext(".hidden"), (".hidden", ""));
}
