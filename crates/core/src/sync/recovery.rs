// crates\core\src\sync
use crate::models::DocId;
use regex::Regex;
use uuid::Uuid;
use tracing::info;

/// Attempts to recover a DocId from the file content using a UUID frontmatter pattern.
/// Returns Some(DocId) if a valid UUID is found.
pub fn try_recover_from_content(content: &str) -> Option<DocId> {
    // Regex to find "uuid: <36-char-uuid>" at start of line
    // (?m) enables multiline mode so ^ matches start of line
    let re = Regex::new(r"(?m)^uuid:\s*([a-fA-F0-9-]{36})").ok()?;
    
    if let Some(caps) = re.captures(content) {
        if let Some(uuid_str) = caps.get(1) {
            if let Ok(uuid_val) = Uuid::parse_str(uuid_str.as_str()) {
                let doc_id = DocId::from_u128(uuid_val.as_u128());
                info!("Recovery: Found UUID in content -> {:?}", doc_id);
                return Some(doc_id);
            }
        }
    }
    
    None
}
