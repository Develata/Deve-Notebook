use super::super::url::{s3_file_url, s3_list_url};
use super::support::{get_header, header, now, test_credentials};

#[test]
fn s3_signed_request_matches_golden_vector() {
    let url = s3_file_url("s3://bucket/notebooks/main", "us-east-1", "a.md").expect("url");
    let request = super::super::signing::signed_put_request(
        url,
        b"a".to_vec(),
        &test_credentials(),
        "us-east-1",
        now(),
    )
    .expect("request");

    assert_eq!(
        header(&request, "x-amz-content-sha256"),
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    );
    assert_eq!(header(&request, "x-amz-date"), "20260705T120000Z");
    assert_eq!(
        header(&request, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260705/us-east-1/s3/aws4_request, SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, Signature=7128a98b8dfe318572aca62bb4f368deb9ca2044a92a7d4a8349e1565f190ffb"
    );
}

#[test]
fn s3_signed_get_request_includes_canonical_query() {
    let url = s3_list_url(
        "s3://bucket/notebooks/main",
        "us-east-1",
        Some("next/token"),
    )
    .expect("url");
    let request = super::super::signing::signed_get_request(
        url,
        &test_credentials(),
        "us-east-1",
        now(),
        4096,
    )
    .expect("request");

    assert_eq!(
        request.url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/?continuation-token=next%2Ftoken&list-type=2&prefix=notebooks%2Fmain%2F"
    );
    assert_eq!(
        get_header(&request, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260705/us-east-1/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=d91974eb4a716deb25cef30fcc8166efa555bde3d48f6a3d9c1b02c1d10f4e26"
    );
}

#[test]
fn s3_signed_request_changes_with_payload() {
    let url = s3_file_url("s3://bucket/notebooks/main", "us-east-1", "a.md").expect("url");
    let left = super::super::signing::signed_put_request(
        url.clone(),
        b"a".to_vec(),
        &test_credentials(),
        "us-east-1",
        now(),
    )
    .expect("left");
    let right = super::super::signing::signed_put_request(
        url,
        b"b".to_vec(),
        &test_credentials(),
        "us-east-1",
        now(),
    )
    .expect("right");

    assert_ne!(
        header(&left, "x-amz-content-sha256"),
        header(&right, "x-amz-content-sha256")
    );
    assert_ne!(
        header(&left, "authorization"),
        header(&right, "authorization")
    );
}
