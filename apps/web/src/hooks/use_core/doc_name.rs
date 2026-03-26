use std::collections::HashSet;

pub fn next_untitled_doc_name<'a, I>(paths: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let mut untitled_taken = false;
    let mut numbered = HashSet::new();

    for path in paths {
        if path == "Untitled.md" {
            untitled_taken = true;
            continue;
        }

        let Some(stem) = path.strip_prefix("Untitled ") else {
            continue;
        };
        let Some(number) = stem.strip_suffix(".md") else {
            continue;
        };
        let Ok(parsed) = number.parse::<u32>() else {
            continue;
        };
        if parsed >= 2 {
            numbered.insert(parsed);
        }
    }

    if !untitled_taken {
        return "Untitled.md".to_string();
    }

    let mut next = 2;
    while numbered.contains(&next) {
        next += 1;
    }
    format!("Untitled {}.md", next)
}

#[cfg(test)]
mod tests {
    use super::next_untitled_doc_name;

    #[test]
    fn prefers_plain_untitled_when_available() {
        assert_eq!(
            next_untitled_doc_name(["notes/a.md", "Untitled 2.md"]),
            "Untitled.md"
        );
    }

    #[test]
    fn increments_when_plain_untitled_exists() {
        assert_eq!(
            next_untitled_doc_name(["Untitled.md", "notes/a.md"]),
            "Untitled 2.md"
        );
    }

    #[test]
    fn fills_first_gap_in_numbered_sequence() {
        assert_eq!(
            next_untitled_doc_name([
                "Untitled.md",
                "Untitled 2.md",
                "Untitled 4.md",
                "nested/Untitled 3.md",
            ]),
            "Untitled 3.md"
        );
    }
}
