use super::{build_new_window_url, build_rename_prefill};

#[test]
fn rename_prefill_keeps_file_in_same_dir() {
    assert_eq!(
        build_rename_prefill("123/324.md"),
        ">mv \"123/324.md\" \"123/324|.md\""
    );
}

#[test]
fn rename_prefill_keeps_folder_without_extension() {
    assert_eq!(
        build_rename_prefill("ABC/modals"),
        ">mv \"ABC/modals\" \"ABC/modals|\""
    );
}

#[test]
fn new_window_url_appends_doc_to_existing_query() {
    assert_eq!(
        build_new_window_url("http://127.0.0.1:8080/?sc_full=1", "123%2F324.md"),
        "http://127.0.0.1:8080/?sc_full=1&doc=123%2F324.md"
    );
}

#[test]
fn new_window_url_preserves_hash_fragment() {
    assert_eq!(
        build_new_window_url("http://127.0.0.1:8080/#section", "doc.md"),
        "http://127.0.0.1:8080/?doc=doc.md#section"
    );
}

#[test]
fn new_window_url_replaces_stale_doc_query_param() {
    assert_eq!(
        build_new_window_url(
            "http://127.0.0.1:8080/?doc=old.md&sc_full=1#section",
            "notes%2Fnew.md"
        ),
        "http://127.0.0.1:8080/?sc_full=1&doc=notes%2Fnew.md#section"
    );
}
