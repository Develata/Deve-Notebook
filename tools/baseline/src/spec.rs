//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Result, bail};

pub fn run_tsv(ctx: &BaselineContext, spec: &str) -> Result<()> {
    for (line_no, raw_line) in spec.lines().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        match parse_operation(line_no + 1, line)? {
            Operation::Contains { rel, text } => ctx.contains(rel, text)?,
            Operation::Absent { rel, text } => ctx.absent(rel, text)?,
            Operation::Before { rel, before, after } => ctx.before(rel, before, after)?,
            Operation::CaseContains {
                acceptance,
                case_id,
                text,
            } => ctx.case_contains(acceptance, case_id, text)?,
            Operation::GitTracked { rel } => ctx.git_tracked(rel)?,
            Operation::GitNotIgnored { rel } => ctx.git_not_ignored(rel)?,
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum Operation<'a> {
    Contains {
        rel: &'a str,
        text: &'a str,
    },
    Absent {
        rel: &'a str,
        text: &'a str,
    },
    Before {
        rel: &'a str,
        before: &'a str,
        after: &'a str,
    },
    CaseContains {
        acceptance: &'a str,
        case_id: &'a str,
        text: &'a str,
    },
    GitTracked {
        rel: &'a str,
    },
    GitNotIgnored {
        rel: &'a str,
    },
}

fn parse_operation(line_no: usize, line: &str) -> Result<Operation<'_>> {
    let fields: Vec<&str> = line.split('\t').collect();
    match fields.as_slice() {
        ["contains", rel, text] => Ok(Operation::Contains { rel, text }),
        ["absent", rel, text] => Ok(Operation::Absent { rel, text }),
        ["before", rel, before, after] => Ok(Operation::Before { rel, before, after }),
        ["case_contains", acceptance, case_id, text] => Ok(Operation::CaseContains {
            acceptance,
            case_id,
            text,
        }),
        ["git_tracked", rel] => Ok(Operation::GitTracked { rel }),
        ["git_not_ignored", rel] => Ok(Operation::GitNotIgnored { rel }),
        [op, ..] => bail!("invalid baseline spec at line {line_no}: unsupported op '{op}'"),
        [] => bail!("invalid baseline spec at line {line_no}: empty line"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Operation, parse_operation};

    #[test]
    fn parses_contains_operation() {
        assert_eq!(
            parse_operation(1, "contains\tCargo.toml\t[workspace]").expect("operation"),
            Operation::Contains {
                rel: "Cargo.toml",
                text: "[workspace]"
            }
        );
    }

    #[test]
    fn rejects_wrong_arity() {
        let error = parse_operation(1, "contains\tCargo.toml").expect_err("invalid");
        assert!(error.to_string().contains("unsupported op"));
    }
}
