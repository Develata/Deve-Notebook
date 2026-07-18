//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-locator-contract
//!   - 06_backup#projection-backup-secret-ref-contract
//!   - 14_commands#cli-commands
//!
//! CLI management commands for host-local Remote Projection profiles.

use super::S3ProjectionProfileAction;
use crate::remote_projection_transport::s3;
use anyhow::Result;
use std::path::Path;

pub(crate) fn run_s3_profile_action(
    ledger_dir: &Path,
    action: &S3ProjectionProfileAction,
) -> Result<()> {
    match action {
        S3ProjectionProfileAction::Put {
            profile,
            endpoint_origin,
            bucket,
            allowed_prefix,
            region,
            credential_env_prefix,
            allowed_capabilities,
        } => {
            let profile = s3::RemoteProjectionS3Profile::env_profile(
                profile,
                endpoint_origin,
                bucket,
                allowed_prefix,
                region,
                credential_env_prefix,
                allowed_capabilities.clone(),
            );
            let path = s3::write_remote_projection_s3_profile(ledger_dir, profile)?;
            println!(
                "projection_remote: wrote host-local secret-free S3 profile store {}",
                path.display()
            );
            Ok(())
        }
        S3ProjectionProfileAction::List => {
            for profile in s3::load_remote_projection_s3_profiles(ledger_dir)? {
                println!(
                    "projection_remote: s3 profile={} endpoint_origin={} bucket={} allowed_prefix={} region={} credential_ref=env_prefix:{} allowed_capabilities={}",
                    profile.profile_id,
                    profile.endpoint_origin,
                    profile.bucket,
                    profile.allowed_prefix,
                    profile.region,
                    profile.credential_ref.env_prefix,
                    profile.allowed_capabilities.join(","),
                );
            }
            Ok(())
        }
    }
}
