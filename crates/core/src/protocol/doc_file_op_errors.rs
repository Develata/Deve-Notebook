pub const DESTINATION_MUST_DIFFER: &str = "Destination must differ from source";
pub const DESTINATION_PATH_REQUIRED: &str = "Destination path required";
pub const DEPTH_LIMIT_EXCEEDED: &str = "Directory depth limit exceeded";
pub const INVALID_EMPTY_PATH: &str = "Invalid empty path";
pub const INVALID_PATH: &str = "Invalid path";
pub const MARKDOWN_DIRECTORY_FORBIDDEN: &str = "Markdown directory is forbidden";
pub const PATH_REQUIRED: &str = "Path required";
pub const RESERVED_INTERNAL_PATH: &str = "Reserved internal path";
pub const SOURCE_PATH_REQUIRED: &str = "Source path required";

pub fn depth_limit_exceeded(max_depth: usize) -> String {
    format!("Directory depth limit exceeded (max {})", max_depth)
}

pub fn invalid_path(path: &str) -> String {
    format!("{}: {}", INVALID_PATH, path)
}

pub fn markdown_directory_forbidden(path: &str) -> String {
    format!("{}: {}", MARKDOWN_DIRECTORY_FORBIDDEN, path)
}

pub fn reserved_internal_path(path: &str) -> String {
    format!("{}: {}", RESERVED_INTERNAL_PATH, path)
}
