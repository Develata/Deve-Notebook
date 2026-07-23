//! plan_ref:
//!   - 14_commands#repo-removal-command-contract
//!
//! Typed process outcomes for CLI contracts that require more precision than
//! the generic failure status. Product detail remains a stable symbolic label.

#[derive(Debug, thiserror::Error)]
#[error("{label}")]
pub(crate) struct CliProcessExit {
    code: u8,
    label: &'static str,
}

impl CliProcessExit {
    pub(crate) const fn new(code: u8, label: &'static str) -> Self {
        Self { code, label }
    }
}

pub fn process_exit_code(error: &anyhow::Error) -> u8 {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<CliProcessExit>())
        .map_or(1, |exit| exit.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_exit_survives_anyhow_context() {
        let error = anyhow::Error::new(CliProcessExit::new(21, "COMMITTED_PARTIAL"))
            .context("outer context");
        assert_eq!(process_exit_code(&error), 21);
        assert_eq!(process_exit_code(&anyhow::anyhow!("generic")), 1);
    }
}
