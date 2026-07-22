//! plan_ref:
//!   - 14_commands#repo-removal-command-contract

use anyhow::{Result, anyhow, bail};
use deve_core::models::RepoId;
use deve_core::protocol::RemovalConfirmationToken;
use uuid::Uuid;

const VERSION: &str = "v1";

pub(super) struct CliRemovalToken {
    pub(super) repo_id: RepoId,
    pub(super) preparation_id: Uuid,
    pub(super) execute_request_id: Uuid,
    pub(super) confirmation: RemovalConfirmationToken,
}

impl CliRemovalToken {
    pub(super) fn issue(
        repo_id: RepoId,
        preparation_id: Uuid,
        confirmation: RemovalConfirmationToken,
    ) -> Self {
        Self {
            repo_id,
            preparation_id,
            execute_request_id: Uuid::new_v4(),
            confirmation,
        }
    }

    pub(super) fn encode(&self) -> String {
        format!(
            "{VERSION}.{}.{}.{}.{}",
            self.repo_id,
            self.preparation_id,
            self.execute_request_id,
            self.confirmation.as_str()
        )
    }

    pub(super) fn parse(value: &str, expected_repo_id: RepoId) -> Result<Self> {
        let fields = value.split('.').collect::<Vec<_>>();
        if fields.len() != 5 || fields[0] != VERSION {
            bail!("REPO_LIFECYCLE_CONFIRMATION_INVALID");
        }
        let repo_id = Uuid::parse_str(fields[1])
            .map_err(|_| anyhow!("REPO_LIFECYCLE_CONFIRMATION_INVALID"))?;
        let preparation_id = Uuid::parse_str(fields[2])
            .map_err(|_| anyhow!("REPO_LIFECYCLE_CONFIRMATION_INVALID"))?;
        let execute_request_id = Uuid::parse_str(fields[3])
            .map_err(|_| anyhow!("REPO_LIFECYCLE_CONFIRMATION_INVALID"))?;
        let confirmation = RemovalConfirmationToken::from_backend(fields[4].to_string())
            .ok_or_else(|| anyhow!("REPO_LIFECYCLE_CONFIRMATION_INVALID"))?;
        if repo_id != expected_repo_id || preparation_id.is_nil() || execute_request_id.is_nil() {
            bail!("REPO_LIFECYCLE_CONFIRMATION_INVALID");
        }
        Ok(Self {
            repo_id,
            preparation_id,
            execute_request_id,
            confirmation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trip_binds_repo_and_redacts_parse_failures() {
        let repo_id = Uuid::new_v4();
        let token = CliRemovalToken::issue(
            repo_id,
            Uuid::new_v4(),
            RemovalConfirmationToken::from_backend("a".repeat(64)).expect("token"),
        );
        let encoded = token.encode();
        let parsed = CliRemovalToken::parse(&encoded, repo_id).expect("round trip");
        assert_eq!(parsed.repo_id, repo_id);
        assert_eq!(parsed.preparation_id, token.preparation_id);
        assert_eq!(parsed.execute_request_id, token.execute_request_id);
        assert!(CliRemovalToken::parse(&encoded, Uuid::new_v4()).is_err());
        let error = match CliRemovalToken::parse("bad", repo_id) {
            Ok(_) => panic!("invalid token must fail"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains(&"a".repeat(64)));
    }
}
