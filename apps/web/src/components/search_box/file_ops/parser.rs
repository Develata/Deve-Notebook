// apps/web/src/components/search_box/file_ops/parser.rs
//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! 参数解析器: 处理引号、空格分隔的命令行参数

#[derive(Clone, Debug)]
pub(super) struct ParsedArgs {
    pub args: Vec<String>,
    pub in_quote: bool,
    pub ends_with_space: bool,
    pub error: Option<ParseError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ParseError {
    PathsWithSpacesMustBeQuoted,
}

pub(super) fn parse_args(input: &str) -> ParsedArgs {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut closed_quote_needs_separator = false;
    let mut error = None;
    let chars = input.chars();
    for ch in chars {
        match ch {
            '"' => {
                if in_quote {
                    in_quote = false;
                    args.push(current.clone());
                    current.clear();
                    closed_quote_needs_separator = true;
                } else if current.is_empty() && !closed_quote_needs_separator {
                    in_quote = true;
                } else {
                    error = Some(ParseError::PathsWithSpacesMustBeQuoted);
                    break;
                }
            }
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                closed_quote_needs_separator = false;
            }
            _ => {
                if closed_quote_needs_separator {
                    error = Some(ParseError::PathsWithSpacesMustBeQuoted);
                    break;
                }
                current.push(ch);
            }
        }
    }
    if error.is_none() && !current.is_empty() {
        args.push(current);
    }
    ParsedArgs {
        args,
        in_quote,
        ends_with_space: input
            .chars()
            .last()
            .map(|c| c.is_whitespace())
            .unwrap_or(false),
        error,
    }
}

pub(super) fn is_ready_for_dst(parsed: &ParsedArgs) -> bool {
    if parsed.args.len() == 1 {
        return parsed.ends_with_space;
    }
    parsed.args.len() == 2
}

pub(super) fn split_command(input: &str) -> Option<(&str, &str)> {
    let mut iter = input.splitn(2, |c: char| c.is_whitespace());
    let cmd = iter.next()?.trim();
    let rest = iter.next().unwrap_or("");
    Some((cmd, rest))
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn parse_args_accepts_whitespace_after_quoted_arg() {
        let parsed = parse_args("\"old name.md\" new.md");

        assert_eq!(parsed.args, vec!["old name.md", "new.md"]);
        assert_eq!(parsed.error, None);
    }

    #[test]
    fn parse_args_rejects_adjacent_text_after_quoted_arg() {
        let parsed = parse_args("\"old name.md\"new.md");

        assert!(parsed.error.is_some());
    }

    #[test]
    fn parse_args_rejects_quote_inside_unquoted_arg() {
        let parsed = parse_args("old\" name.md\" new.md");

        assert!(parsed.error.is_some());
    }
}
