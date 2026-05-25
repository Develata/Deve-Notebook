//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! HTTP query helpers shared by Web API adapters.

pub(super) fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => push_percent_encoded_byte(&mut encoded, byte),
        }
    }
    encoded
}

fn push_percent_encoded_byte(encoded: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    encoded.push('%');
    encoded.push(HEX[(byte >> 4) as usize] as char);
    encoded.push(HEX[(byte & 0x0F) as usize] as char);
}

#[cfg(test)]
mod tests {
    use super::encode_query_component;

    #[test]
    fn query_component_encoding_preserves_unreserved_bytes() {
        assert_eq!(encode_query_component("AZaz09-._~"), "AZaz09-._~");
    }

    #[test]
    fn query_component_encoding_escapes_reserved_and_utf8_bytes() {
        assert_eq!(
            encode_query_component("repo 1&x=1/雪"),
            "repo%201%26x%3D1%2F%E9%9B%AA"
        );
    }
}
