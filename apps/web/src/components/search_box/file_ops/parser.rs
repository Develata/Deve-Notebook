// apps/web/src/components/search_box/file_ops/parser.rs
//! 参数解析器: 处理引号、空格分隔的命令行参数

#[derive(Clone, Debug)]
pub(super) struct ParsedArgs {
    pub args: Vec<String>,
    pub in_quote: bool,
    pub ends_with_space: bool,
    pub error: Option<String>,
}

pub(super) fn parse_args(input: &str) -> ParsedArgs {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let chars = input.chars().peekable();
    for ch in chars {
        match ch {
            '"' => {
                in_quote = !in_quote;
                if !in_quote {
                    args.push(current.clone());
                    current.clear();
                }
            }
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
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
        error: None,
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
