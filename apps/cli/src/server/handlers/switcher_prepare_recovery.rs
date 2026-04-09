pub(super) fn should_degrade_local_projection(err: &anyhow::Error) -> bool {
    let lower = err.to_string().to_ascii_lowercase();
    lower.contains("structure projection references missing parent")
        || lower.contains("structure projection rename references missing node")
        || lower.contains("structure projection move references missing node")
        || lower.contains("structure projection contains cycle")
        || lower.contains("structure projection lost doc identity")
}
