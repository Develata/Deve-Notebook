use super::super::url::{webdav_file_url, webdav_locator_to_https_url};

#[test]
fn webdav_locator_scheme_matching_is_case_insensitive() {
    let base = webdav_locator_to_https_url("WEBDAV+HTTPS://dav.example.com/notebooks/main")
        .expect("uppercase WebDAV locator");
    assert_eq!(base.as_str(), "https://dav.example.com/notebooks/main");

    let file_url = webdav_file_url(&base, "notes/a.md").expect("file URL");
    assert_eq!(
        file_url.as_str(),
        "https://dav.example.com/notebooks/main/notes/a.md"
    );
}
