/// Markdown/text formatting utilities

/// Indent every line of text by the given number of spaces
pub fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a module path as a tree entry with indentation
pub fn tree_entry(path: &str, description: &str, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let short_name = path.rsplit("::").next().unwrap_or(path);
    if description.is_empty() {
        format!("{}- {}", indent, short_name)
    } else {
        format!("{}- {} — {}", indent, short_name, description)
    }
}

/// Strip the "crate::" prefix from a module path for display
pub fn display_module_path(path: &str) -> &str {
    path.strip_prefix("crate::").unwrap_or(path)
}

/// Truncate a string to a maximum length, adding "..." if truncated
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format a code block in markdown
pub fn code_block(code: &str, language: &str) -> String {
    format!("```{}\n{}\n```", language, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent() {
        assert_eq!(indent("hello\nworld", 4), "    hello\n    world");
    }

    #[test]
    fn test_tree_entry() {
        assert_eq!(
            tree_entry("crate::engine::eval", "Expression evaluator", 2),
            "    - eval — Expression evaluator"
        );
    }

    #[test]
    fn test_display_module_path() {
        assert_eq!(display_module_path("crate::engine::eval"), "engine::eval");
        assert_eq!(display_module_path("crate"), "crate");
    }
}
