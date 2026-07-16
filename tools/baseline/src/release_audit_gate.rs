//! plan_ref:
//!   - 23_threat_model#supply-chain
//!   - 18_release#first-tag-acceptance-matrix

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
            let registry = registry::read_registry(ctx.root())?;
            validate_tag_ready_if_required(&flags, &registry)?;
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
            validate_tag_ready_if_required(&flags, &registry)?;
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

fn validate_tag_ready_if_required(flags: &AuditFlags, registry: &str) -> Result<()> {
    if flags.tag_ready_required {
        registry::validate_no_tag_blockers(registry)?;
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
    tag_ready_required: bool,
}

impl AuditFlags {
    fn from_env() -> Result<Self> {
        Ok(Self {
            release_required: binary_flag_from_env(LABEL, "DEVE_RELEASE_AUDIT_REQUIRED", false)?,
            cargo_required: binary_flag_from_env(LABEL, "DEVE_CARGO_AUDIT_REQUIRED", false)?,
            npm_required: binary_flag_from_env(LABEL, "DEVE_NPM_AUDIT_REQUIRED", false)?,
            tag_ready_required: binary_flag_from_env(
                LABEL,
                "DEVE_RELEASE_TAG_READY_REQUIRED",
                false,
            )?,
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
    use super::{AuditFlags, LABEL, run, validate_tag_ready_if_required};
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
            tag_ready_required: false,
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
            tag_ready_required: false,
        };

        assert!(flags.cargo_required());
        assert!(!flags.npm_required());
    }

    #[test]
    fn parses_release_tag_ready_required_flags() {
        assert!(!parse_binary_flag(LABEL, "DEVE_RELEASE_TAG_READY_REQUIRED", "0").expect("flag"));
        assert!(parse_binary_flag(LABEL, "DEVE_RELEASE_TAG_READY_REQUIRED", "1").expect("flag"));
    }

    #[test]
    fn tag_ready_allows_registry_after_first_tag_blockers_are_resolved() {
        run(&[]).expect("default release audit registry gate should pass");
        run(&["tag-ready".to_string()])
            .expect("first-tag gate should pass after ADR 0006 Route 2 resolves blockers");
    }

    #[test]
    fn tag_ready_env_gate_rejects_blockers_for_shared_actions() {
        let flags = AuditFlags {
            release_required: false,
            cargo_required: false,
            npm_required: false,
            tag_ready_required: true,
        };
        let registry = "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | yes | reason | route |\n";

        let error = validate_tag_ready_if_required(&flags, registry).expect_err("tag blocker");
        assert!(
            error.to_string().contains("first-tag readiness is blocked"),
            "{error}"
        );
    }
}
