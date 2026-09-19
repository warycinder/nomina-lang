use crate::parser::{Expr, Grammar, Rule};

const MAX_LINE_WIDTH: usize = 80;

/// Reformats a grammar into a single canonical layout: one space around
/// `=` and `|`, no space before `;`, and alternatives that don't fit on
/// one line broken out one per line with leading `|` markers.
pub fn pretty_print(grammar: &Grammar) -> String {
    let mut out = String::new();
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format_rule(rule));
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

fn needs_parens_around_opt(expr: &Expr) -> bool {
    matches!(expr, Expr::Alt(_) | Expr::Seq(_))
}

fn needs_parens_in_seq(expr: &Expr) -> bool {
    matches!(expr, Expr::Alt(_))
}
