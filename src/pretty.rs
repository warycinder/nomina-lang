use crate::lexer::Comment;
use crate::parser::{Expr, Grammar, Rule};

const MAX_LINE_WIDTH: usize = 80;

/// Reformats a grammar into a single canonical layout: one space around
/// `=` and `|`, no space before `;`, and alternatives that don't fit on
/// one line broken out one per line with leading `|` markers.
///
/// Comments are not part of the syntax tree, so they are matched to rules
/// by line number. A comment above a rule stays above it. A lone comment
/// at the end of a rule's line stays there if the rule still fits on one
/// line; any other comment inside a rule's span is moved to just above
/// the rule, since reflowing can change which line it would land on.
/// Comments after the last rule stay at the end of the file.
pub fn pretty_print(grammar: &Grammar, comments: &[Comment]) -> String {
    let mut out = String::new();
    let mut next = 0;
    for (i, rule) in grammar.rules.iter().enumerate() {
        let start = rule.name_span.line;
        let taken = comments[next..]
            .iter()
            .take_while(|c| c.line <= rule.end_line)
            .count();
        let group = &comments[next..next + taken];
        next += taken;

        let mut body = format_rule(rule);
        let inside: Vec<&Comment> = group.iter().filter(|c| c.line >= start).collect();
        let single_line = body.matches('\n').count() == 1;
        let inline_note = match inside.as_slice() {
            [only] if only.trailing && single_line && only.line == rule.end_line => {
                Some(*only)
            }
            _ => None,
        };
        if let Some(note) = inline_note {
            body.pop();
            body.push_str(&format!("  #{}\n", note.text));
        }

        if i > 0 {
            out.push('\n');
        }
        for c in group {
            let is_inline = inline_note.map_or(false, |n| std::ptr::eq(n, c));
            if !is_inline {
                out.push_str(&format!("#{}\n", c.text));
            }
        }
        out.push_str(&body);
    }

    if next < comments.len() {
        if !grammar.rules.is_empty() {
            out.push('\n');
        }
        for c in &comments[next..] {
            out.push_str(&format!("#{}\n", c.text));
        }
    }
    out
}

fn format_rule(rule: &Rule) -> String {
    let body = format_expr(&rule.expr);
    let inline = format!("{} = {};\n", rule.name, body);
    if inline.chars().count() <= MAX_LINE_WIDTH {
        return inline;
    }

    match &rule.expr {
        Expr::Alt(branches) => {
            let mut out = format!("{} =\n", rule.name);
            for (i, branch) in branches.iter().enumerate() {
                let prefix = if i == 0 { "    " } else { "    | " };
                out.push_str(prefix);
                out.push_str(&format_expr(branch));
                out.push('\n');
            }
            out.push_str("    ;\n");
            out
        }
        _ => inline,
    }
}

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(s, _) => format!("\"{}\"", s),
        Expr::Ref(name, _) => name.clone(),
        Expr::Opt(inner) => {
            let s = format_expr(inner);
            if needs_parens_around_opt(inner) {
                format!("({})?", s)
            } else {
                format!("{}?", s)
            }
        }
        Expr::Seq(items) => items
            .iter()
            .map(|item| {
                let s = format_expr(item);
                if needs_parens_in_seq(item) {
                    format!("({})", s)
                } else {
                    s
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        Expr::Alt(branches) => branches
            .iter()
            .map(format_expr)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn fmt(source: &str) -> String {
        let (tokens, comments) = Lexer::new(source).tokenize().unwrap();
        let grammar = Parser::new(tokens).parse_grammar().unwrap();
        pretty_print(&grammar, &comments)
    }

    #[test]
    fn keeps_comment_above_rule() {
        let out = fmt("# the entry point\nname=\"a\"|\"b\";\n");
        assert_eq!(out, "# the entry point\nname = \"a\" | \"b\";\n");
    }

    #[test]
    fn keeps_trailing_comment_on_one_line_rule() {
        let out = fmt("name = \"a\"; # just a\n");
        assert_eq!(out, "name = \"a\";  # just a\n");
    }

    #[test]
    fn hash_inside_string_is_not_a_comment() {
        let out = fmt("name = \"#1\";\n");
        assert_eq!(out, "name = \"#1\";\n");
    }

    #[test]
    fn moves_comment_inside_rule_above_it() {
        let out = fmt("name = \"a\" # first\n  | \"b\";\n");
        assert_eq!(out, "# first\nname = \"a\" | \"b\";\n");
    }

    #[test]
    fn keeps_comments_after_last_rule() {
        let out = fmt("name = \"a\";\n\n# todo: more\n");
        assert_eq!(out, "name = \"a\";\n\n# todo: more\n");
    }

    #[test]
    fn separates_rules_with_blank_line() {
        let out = fmt("name = x;\n# the x rule\nx = \"x\";\n");
        assert_eq!(out, "name = x;\n\n# the x rule\nx = \"x\";\n");
    }
}

fn needs_parens_around_opt(expr: &Expr) -> bool {
    matches!(expr, Expr::Alt(_) | Expr::Seq(_))
}

fn needs_parens_in_seq(expr: &Expr) -> bool {
    matches!(expr, Expr::Alt(_))
}
