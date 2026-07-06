//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunMode {
    TextOnly,
    Full,
}

pub fn run_tsv(ctx: &BaselineContext, spec: &str) -> Result<()> {
    run_tsv_with_mode(ctx, spec, RunMode::Full)
}

pub fn run_tsv_with_mode(ctx: &BaselineContext, spec: &str, mode: RunMode) -> Result<()> {
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
            Operation::CheckScriptsListed { rel } => ctx.check_scripts_listed(rel)?,
            Operation::AbsentOptional { rel, text } => ctx.absent_optional(rel, text)?,
            Operation::AbsentTree { rel, text } => ctx.absent_tree(rel, text)?,
            Operation::AbsentTreeSkipTests { rel, text } => {
                ctx.absent_tree_skip_tests(rel, text)?
            }
            Operation::RegexAbsent { rel, pattern } => ctx.regex_absent(rel, pattern)?,
            Operation::RegexAbsentTree {
                rel,
                pattern,
                include_ext,
                skip_suffixes,
            } => ctx.regex_absent_tree(rel, pattern, include_ext, &skip_suffixes)?,
            Operation::CssNumberLt { rel, left, right } => ctx.css_number_lt(rel, left, right)?,
            Operation::GitTracked { rel } => ctx.git_tracked(rel)?,
            Operation::GitNotIgnored { rel } => ctx.git_not_ignored(rel)?,
            Operation::CargoTest { package, filter } => {
                if mode == RunMode::Full {
                    ctx.cargo_test(package, filter)?;
                }
            }
            Operation::CargoTestLib { package, filter } => {
                if mode == RunMode::Full {
                    ctx.cargo_test_lib(package, filter)?;
                }
            }
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
    CheckScriptsListed {
        rel: &'a str,
    },
    AbsentOptional {
        rel: &'a str,
        text: &'a str,
    },
    AbsentTree {
        rel: &'a str,
        text: &'a str,
    },
    AbsentTreeSkipTests {
        rel: &'a str,
        text: &'a str,
    },
    RegexAbsent {
        rel: &'a str,
        pattern: &'a str,
    },
    RegexAbsentTree {
        rel: &'a str,
        pattern: &'a str,
        include_ext: Option<&'a str>,
        skip_suffixes: Vec<&'a str>,
    },
    CssNumberLt {
        rel: &'a str,
        left: &'a str,
        right: &'a str,
    },
    GitTracked {
        rel: &'a str,
    },
    GitNotIgnored {
        rel: &'a str,
    },
    CargoTest {
        package: &'a str,
        filter: &'a str,
    },
    CargoTestLib {
        package: &'a str,
        filter: &'a str,
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
        ["check_scripts_listed", rel] => Ok(Operation::CheckScriptsListed { rel }),
        ["absent_optional", rel, text] => Ok(Operation::AbsentOptional { rel, text }),
        ["absent_tree", rel, text] => Ok(Operation::AbsentTree { rel, text }),
        ["absent_tree_skip_tests", rel, text] => Ok(Operation::AbsentTreeSkipTests { rel, text }),
        ["regex_absent", rel, pattern] => Ok(Operation::RegexAbsent { rel, pattern }),
        ["regex_absent_tree", rel, pattern] => Ok(Operation::RegexAbsentTree {
            rel,
            pattern,
            include_ext: None,
            skip_suffixes: Vec::new(),
        }),
        ["regex_absent_tree_ext", rel, ext, pattern] => Ok(Operation::RegexAbsentTree {
            rel,
            pattern,
            include_ext: Some(ext),
            skip_suffixes: Vec::new(),
        }),
        ["regex_absent_tree_skip", rel, pattern, skip_suffixes] => Ok(Operation::RegexAbsentTree {
            rel,
            pattern,
            include_ext: None,
            skip_suffixes: split_list(skip_suffixes),
        }),
        [
            "regex_absent_tree_ext_skip",
            rel,
            ext,
            pattern,
            skip_suffixes,
        ] => Ok(Operation::RegexAbsentTree {
            rel,
            pattern,
            include_ext: Some(ext),
            skip_suffixes: split_list(skip_suffixes),
        }),
        ["css_number_lt", rel, left, right] => Ok(Operation::CssNumberLt { rel, left, right }),
        ["git_tracked", rel] => Ok(Operation::GitTracked { rel }),
        ["git_not_ignored", rel] => Ok(Operation::GitNotIgnored { rel }),
        ["cargo_test", package, filter] => Ok(Operation::CargoTest { package, filter }),
        ["cargo_test_lib", package, filter] => Ok(Operation::CargoTestLib { package, filter }),
        [op, ..] => {
            if let Some(expected) = expected_field_count(op) {
                bail!(
                    "invalid baseline spec at line {line_no}: op '{op}' expected {expected} tab-separated fields, got {}",
                    fields.len()
                )
            }
            bail!("invalid baseline spec at line {line_no}: unsupported op '{op}'")
        }
        [] => bail!("invalid baseline spec at line {line_no}: empty line"),
    }
}

fn expected_field_count(op: &str) -> Option<usize> {
    Some(match op {
        "contains"
        | "absent"
        | "absent_optional"
        | "absent_tree"
        | "absent_tree_skip_tests"
        | "regex_absent"
        | "regex_absent_tree" => 3,
        "before" | "case_contains" | "regex_absent_tree_ext" | "css_number_lt" => 4,
        "check_scripts_listed" | "git_tracked" | "git_not_ignored" => 2,
        "regex_absent_tree_skip" => 4,
        "regex_absent_tree_ext_skip" => 5,
        "cargo_test" | "cargo_test_lib" => 3,
        _ => return None,
    })
}

fn split_list(value: &str) -> Vec<&str> {
    value.split('|').filter(|item| !item.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::{Operation, parse_operation, split_list};

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
        assert!(
            error
                .to_string()
                .contains("op 'contains' expected 3 tab-separated fields, got 2")
        );
    }

    #[test]
    fn rejects_unknown_operation() {
        let error = parse_operation(1, "unknown_op\tCargo.toml").expect_err("invalid");
        assert!(error.to_string().contains("unsupported op 'unknown_op'"));
    }

    #[test]
    fn parses_regex_tree_with_skips() {
        assert_eq!(
            parse_operation(
                1,
                "regex_absent_tree_ext_skip\tapps/web/style\tcss\t#([0-9a-fA-F]{3,6})\t_variables.css|_variables-dark.css"
            )
            .expect("operation"),
            Operation::RegexAbsentTree {
                rel: "apps/web/style",
                pattern: "#([0-9a-fA-F]{3,6})",
                include_ext: Some("css"),
                skip_suffixes: vec!["_variables.css", "_variables-dark.css"],
            }
        );
    }

    #[test]
    fn split_list_ignores_empty_items() {
        assert_eq!(split_list("a||b|"), vec!["a", "b"]);
    }

    #[test]
    fn parses_cargo_test_operation() {
        assert_eq!(
            parse_operation(1, "cargo_test\tdeve_web\tsettings").expect("operation"),
            Operation::CargoTest {
                package: "deve_web",
                filter: "settings",
            }
        );
    }

    #[test]
    fn parses_cargo_test_lib_operation() {
        assert_eq!(
            parse_operation(1, "cargo_test_lib\tdeve_core\tsettings").expect("operation"),
            Operation::CargoTestLib {
                package: "deve_core",
                filter: "settings",
            }
        );
    }
}
