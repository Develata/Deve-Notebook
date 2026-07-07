//! plan_ref: infra

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use crate::spec::run_tsv;
use anyhow::{Result, bail};
use std::fs;
use std::path::Path;

mod registry;

const LABEL: &str = "release-audit-gate";
const CARGO_AUDIT_UNAVAILABLE: &str = "cargo-audit unavailable; install with 'cargo install cargo-audit --locked' or set DEVE_CARGO_AUDIT_REQUIRED=0 for local diagnostic-only runs";
const NPM_UNAVAILABLE: &str = "npm unavailable; install Node.js/npm or set DEVE_NPM_AUDIT_REQUIRED=0 for local diagnostic-only runs";

pub fn run(args: &[String]) -> Result<()> {
    let flags = AuditFlags::from_env()?;
    match args {
        [] => {
            let ctx = BaselineContext::new(LABEL)?;
            run_tsv(&ctx, include_str!("specs/release_audit_gate.tsv"))?;
            registry::validate_registry_file(ctx.root())?;
            ctx.ok();
        }
        [action] if action == "cargo-audit-missing" => {
            report_missing_tool(
                flags.cargo_required(),
                "cargo audit",
                CARGO_AUDIT_UNAVAILABLE,
            )?;
        }
        [action] if action == "npm-audit-missing" => {
            report_missing_tool(flags.npm_required(), "npm audit", NPM_UNAVAILABLE)?;
        }
        [action, report] if action == "cargo-audit-report" => {
            let ctx = BaselineContext::new(LABEL)?;
            run_tsv(&ctx, include_str!("specs/release_audit_gate.tsv"))?;
            let registry = registry::read_registry(ctx.root())?;
            let report_path = Path::new(report);
            let report_path = if report_path.is_absolute() {
                report_path.to_path_buf()
            } else {
                ctx.root().join(report_path)
            };
            let report = fs::read_to_string(&report_path).map_err(|err| {
                anyhow::anyhow!(
                    "{LABEL}: failed to read cargo audit report {}: {err}",
                    report_path.display()
                )
            })?;
            registry::validate_cargo_audit_report(&report, &registry)?;
        }
        [action] if action == "tag-ready" => {
            let ctx = BaselineContext::new(LABEL)?;
            let registry = registry::read_registry(ctx.root())?;
            registry::validate_no_tag_blockers(&registry)?;
        }
        [action] => bail!("{LABEL}: unsupported release audit action: {action}"),
        _ => bail!("{LABEL}: expected zero or one release audit action"),
    }
    Ok(())
}

fn report_missing_tool(required: bool, audit_label: &str, message: &str) -> Result<()> {
    if required {
        bail!("{LABEL}: {message}");
    }
    eprintln!("{LABEL}: skip {audit_label}: {message}");
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct AuditFlags {
    release_required: bool,
    cargo_required: bool,
    npm_required: bool,
}

impl AuditFlags {
    fn from_env() -> Result<Self> {
        Ok(Self {
            release_required: binary_flag_from_env(LABEL, "DEVE_RELEASE_AUDIT_REQUIRED", false)?,
            cargo_required: binary_flag_from_env(LABEL, "DEVE_CARGO_AUDIT_REQUIRED", false)?,
            npm_required: binary_flag_from_env(LABEL, "DEVE_NPM_AUDIT_REQUIRED", false)?,
        })
    }

    fn cargo_required(&self) -> bool {
        self.release_required || self.cargo_required
    }

    fn npm_required(&self) -> bool {
        self.release_required || self.npm_required
    }
}

#[cfg(test)]
mod tests {
    use super::{AuditFlags, LABEL};
    use crate::env_gate::parse_binary_flag;

    #[test]
    fn parses_release_audit_required_flags() {
        assert!(!parse_binary_flag(LABEL, "DEVE_RELEASE_AUDIT_REQUIRED", "0").expect("flag"));
        assert!(parse_binary_flag(LABEL, "DEVE_RELEASE_AUDIT_REQUIRED", "1").expect("flag"));
    }

    #[test]
    fn rejects_non_binary_release_audit_flags() {
        for value in ["", "true", "false", "yes", "2"] {
            assert!(parse_binary_flag(LABEL, "DEVE_RELEASE_AUDIT_REQUIRED", value).is_err());
        }
    }

    #[test]
    fn global_release_audit_required_forces_component_audits() {
        let flags = AuditFlags {
            release_required: true,
            cargo_required: false,
            npm_required: false,
        };

        assert!(flags.cargo_required());
        assert!(flags.npm_required());
    }

    #[test]
    fn component_release_audit_required_stays_component_scoped() {
        let flags = AuditFlags {
            release_required: false,
            cargo_required: true,
            npm_required: false,
        };

        assert!(flags.cargo_required());
        assert!(!flags.npm_required());
    }
}
