#[path = "providers_branch.rs"]
mod providers_branch;
#[path = "providers_command.rs"]
mod providers_command;
#[path = "providers_file.rs"]
mod providers_file;

pub use self::providers_branch::{BranchProvider, LOCAL_BRANCH_LABEL};
pub use self::providers_command::CommandProvider;
pub use self::providers_file::FileProvider;
