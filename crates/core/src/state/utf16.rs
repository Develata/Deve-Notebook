//! plan_ref:
//!   - 10_rendering#large-document-runtime

pub(super) fn utf16_len(text: &str) -> Option<u32> {
    u32::try_from(text.encode_utf16().count()).ok()
}
