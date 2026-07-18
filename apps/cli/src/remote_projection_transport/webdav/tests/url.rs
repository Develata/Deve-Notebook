use super::super::url::{relative_path_from_href, webdav_file_url, webdav_locator_to_https_url};

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

#[test]
fn webdav_href_decodes_once_and_trailing_locator_does_not_double_slash() {
    let base = webdav_locator_to_https_url("webdav+https://dav.example.com/notebooks/main/")
        .expect("trailing-slash locator");
    let path = relative_path_from_href(
        &base,
        "https://dav.example.com/notebooks/main/notes/a%20%E6%96%87.md",
    )
    .expect("href")
    .expect("relative path");
    assert_eq!(path, "notes/a 文.md");

    let file_url = webdav_file_url(&base, &path).expect("file URL");
    assert_eq!(
        file_url.as_str(),
        "https://dav.example.com/notebooks/main/notes/a%20%E6%96%87.md"
    );
}

#[test]
fn webdav_href_rejects_encoded_path_separators() {
    let base =
        webdav_locator_to_https_url("webdav+https://dav.example.com/notebooks/main").expect("base");
    for href in [
        "https://dav.example.com/notebooks/main/notes%2Fa.md",
        "https://dav.example.com/notebooks/main/notes%5Ca.md",
    ] {
        assert!(relative_path_from_href(&base, href).is_err(), "{href}");
    }
}

#[test]
fn webdav_href_rejects_foreign_origin_and_collection() {
    let base =
        webdav_locator_to_https_url("webdav+https://dav.example.com/notebooks/main").expect("base");
    for href in [
        "https://evil.example.com/notebooks/main/a.md",
        "https://dav.example.com/notebooks/other/a.md",
    ] {
        assert!(relative_path_from_href(&base, href).is_err(), "{href}");
    }
    assert_eq!(
        relative_path_from_href(&base, "https://dav.example.com/notebooks/main")
            .expect("base self"),
        None
    );
}
