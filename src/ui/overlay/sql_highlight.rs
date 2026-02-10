use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::ui::theme::Theme;

/// SQL keywords to highlight
pub const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "AS",
    "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "ON", "USING",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "TRUNCATE",
    "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "SCHEMA", "DATABASE",
    "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "CHECK", "DEFAULT",
    "CONSTRAINT", "CASCADE", "RESTRICT", "GRANT", "REVOKE", "COMMIT", "ROLLBACK",
    "BEGIN", "END", "TRANSACTION", "SAVEPOINT", "RELEASE",
    "ORDER", "BY", "ASC", "DESC", "NULLS", "FIRST", "LAST",
    "GROUP", "HAVING", "LIMIT", "OFFSET", "FETCH", "NEXT", "ROWS", "ONLY",
    "UNION", "INTERSECT", "EXCEPT", "ALL", "DISTINCT", "EXISTS",
    "CASE", "WHEN", "THEN", "ELSE", "COALESCE", "NULLIF", "CAST",
    "TRUE", "FALSE", "LIKE", "ILIKE", "SIMILAR", "BETWEEN", "ANY", "SOME",
    "WITH", "RECURSIVE", "RETURNING", "CONFLICT", "DO", "NOTHING",
    "OVER", "PARTITION", "WINDOW", "FILTER", "WITHIN", "LATERAL",
    "FOR", "SHARE", "NOWAIT", "SKIP", "LOCKED",
    "EXPLAIN", "ANALYZE", "VERBOSE", "COSTS", "BUFFERS", "TIMING", "FORMAT",
    "VACUUM", "REINDEX", "CLUSTER", "REFRESH", "MATERIALIZED",
    "TRIGGER", "FUNCTION", "PROCEDURE", "RETURNS", "LANGUAGE", "SECURITY", "DEFINER",
    "IF", "THEN", "ELSIF", "LOOP", "WHILE", "EXIT", "CONTINUE", "RETURN",
    "DECLARE", "VARIABLE", "CONSTANT", "CURSOR", "EXCEPTION", "RAISE", "PERFORM",
    "EXECUTE", "PREPARE", "DEALLOCATE",
];

/// Highlight SQL syntax for inline display (single line, for table cells)
/// Collapses whitespace and truncates to max_len
pub fn highlight_sql_inline(text: &str, max_len: usize) -> Vec<Span<'static>> {
    let keyword_style = Style::default().fg(Theme::sql_keyword());
    let string_style = Style::default().fg(Theme::sql_string());
    let number_style = Style::default().fg(Theme::sql_number());
    let default_style = Style::default().fg(Theme::fg());

    // Collapse whitespace and truncate (Unicode-safe)
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let display: String = if collapsed.chars().count() > max_len {
        collapsed.chars().take(max_len).collect()
    } else {
        collapsed.clone()
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = display.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Check for string literal
        if c == '\'' {
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, string_style));
            continue;
        }

        // Check for positional parameter $N
        if c == '$' && i + 1 < len && chars[i + 1].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, default_style));
            continue;
        }

        // Check for number
        if c.is_ascii_digit() || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            spans.push(Span::styled(num, number_style));
            continue;
        }

        // Check for identifier/keyword
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            let style = if SQL_KEYWORDS.contains(&upper.as_str()) {
                keyword_style
            } else {
                default_style
            };
            spans.push(Span::styled(word, style));
            continue;
        }

        // Any other characters (whitespace, operators, punctuation)
        let start = i;
        while i < len {
            let ch = chars[i];
            if ch.is_alphabetic()
                || ch == '_'
                || ch.is_ascii_digit()
                || ch == '\''
                || ch == '$'
            {
                break;
            }
            i += 1;
        }
        if i == start {
            i += 1;
        }
        let other: String = chars[start..i].iter().collect();
        spans.push(Span::styled(other, default_style));
    }

    spans
}

/// Highlight SQL syntax in the given text, returning styled spans
pub(super) fn highlight_sql(text: &str, indent: &str) -> Vec<Line<'static>> {
    let keyword_style = Style::default().fg(Theme::sql_keyword());
    let string_style = Style::default().fg(Theme::sql_string());
    let number_style = Style::default().fg(Theme::sql_number());
    let comment_style = Style::default().fg(Theme::sql_comment());
    let default_style = Style::default().fg(Theme::fg());

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = vec![Span::styled(indent.to_string(), default_style)];

    // Helper to push a styled string, splitting on newlines
    let push_styled = |s: String, style: Style, spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>, indent: &str, default_style: Style| {
        let parts: Vec<&str> = s.split('\n').collect();
        for (idx, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                spans.push(Span::styled(part.to_string(), style));
            }
            if idx < parts.len() - 1 {
                // There's a newline after this part
                lines.push(Line::from(std::mem::take(spans)));
                spans.push(Span::styled(indent.to_string(), default_style));
            }
        }
    };

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Check for single-line comment --
        if c == '-' && i + 1 < len && chars[i + 1] == '-' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let comment: String = chars[start..i].iter().collect();
            push_styled(comment, comment_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for multi-line comment /* */
        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip */
            }
            let comment: String = chars[start..i].iter().collect();
            push_styled(comment, comment_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for string literal
        if c == '\'' {
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        i += 2; // escaped quote
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            push_styled(s, string_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for dollar-quoted string $tag$...$tag$
        if c == '$' {
            let tag_start = i;
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            if i < len && chars[i] == '$' {
                i += 1;
                let tag: String = chars[tag_start..i].iter().collect();
                // Find closing tag
                while i < len {
                    if chars[i] == '$' {
                        let mut matches = true;
                        for (j, tc) in tag.chars().enumerate() {
                            if i + j >= len || chars[i + j] != tc {
                                matches = false;
                                break;
                            }
                        }
                        if matches {
                            i += tag.len();
                            break;
                        }
                    }
                    i += 1;
                }
                let s: String = chars[tag_start..i].iter().collect();
                push_styled(s, string_style, &mut current_spans, &mut lines, indent, default_style);
                continue;
            } else {
                // Check if it's a positional parameter like $1, $23
                let scanned: String = chars[tag_start..i].iter().collect();
                if scanned.len() > 1 && scanned[1..].chars().all(|ch| ch.is_ascii_digit()) {
                    // It's a positional parameter - treat as default style
                    push_styled(scanned, default_style, &mut current_spans, &mut lines, indent, default_style);
                    continue;
                }
                // Not a dollar-quoted string or parameter, just a $
                i = tag_start;
            }
        }

        // Check for number
        if c.is_ascii_digit() || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'e' || chars[i] == 'E' || chars[i] == '+' || chars[i] == '-') {
                // Handle scientific notation carefully
                if (chars[i] == '+' || chars[i] == '-') && i > start {
                    let prev = chars[i - 1];
                    if prev != 'e' && prev != 'E' {
                        break;
                    }
                }
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            push_styled(num, number_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for identifier/keyword
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            let style = if SQL_KEYWORDS.contains(&upper.as_str()) {
                keyword_style
            } else {
                default_style
            };
            push_styled(word, style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Any other characters (whitespace, operators, punctuation)
        let start = i;
        while i < len {
            let ch = chars[i];
            // Stop if we hit something that needs special handling (including newline)
            if ch == '\n'
                || ch.is_alphabetic()
                || ch == '_'
                || ch.is_ascii_digit()
                || ch == '\''
                || (ch == '-' && i + 1 < len && chars[i + 1] == '-')
                || (ch == '/' && i + 1 < len && chars[i + 1] == '*')
                || (ch == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
            {
                break;
            }
            i += 1;
        }
        // Always make progress - if nothing matched, take at least one char
        // (handles edge cases like standalone $ that isn't a dollar-quote)
        if i == start {
            i += 1;
        }
        let other: String = chars[start..i].iter().collect();
        push_styled(other, default_style, &mut current_spans, &mut lines, indent, default_style);
    }

    // Push any remaining spans as the final line
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(indent.to_string(), default_style)));
    }

    lines
}
