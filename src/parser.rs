use std::collections::{HashMap, HashSet};

use crate::error::{Diagnostic, Span};
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct Grammar {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub name_span: Span,
    /// Line of the closing `;`, so comments can be matched to the rule
    /// that spans them.
    pub end_line: usize,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(String, Span),
    Ref(String, Span),
    Seq(Vec<Expr>),
    Alt(Vec<Expr>),
    Opt(Box<Expr>),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: TokenKind, what: &str) -> Result<Token, Diagnostic> {
        if self.peek().kind == kind {
            Ok(self.bump())
        } else {
            Err(Diagnostic {
                message: format!("expected {}, found {}", what, describe(&self.peek().kind)),
                span: self.peek().span,
                note: None,
            })
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<Token, Diagnostic> {
        match &self.peek().kind {
            TokenKind::Ident(_) => Ok(self.bump()),
            _ => Err(Diagnostic {
                message: format!("expected {}, found {}", what, describe(&self.peek().kind)),
                span: self.peek().span,
                note: None,
            }),
        }
    }

    pub fn parse_grammar(&mut self) -> Result<Grammar, Diagnostic> {
        let mut rules = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            rules.push(self.parse_rule()?);
        }
        Ok(Grammar { rules })
    }

    fn parse_rule(&mut self) -> Result<Rule, Diagnostic> {
        let name_tok = self.expect_ident("a rule name")?;
        let name = match name_tok.kind {
            TokenKind::Ident(s) => s,
            _ => unreachable!(),
        };
        self.expect(TokenKind::Equals, "`=`")?;
        let expr = self.parse_alt()?;
        let semi = self.expect(TokenKind::Semicolon, "`;`")?;
        Ok(Rule { name, name_span: name_tok.span, end_line: semi.span.line, expr })
    }

    fn parse_alt(&mut self) -> Result<Expr, Diagnostic> {
        let mut branches = vec![self.parse_seq()?];
        while self.peek().kind == TokenKind::Pipe {
            self.bump();
            branches.push(self.parse_seq()?);
        }
        if branches.len() == 1 {
            Ok(branches.into_iter().next().unwrap())
        } else {
            Ok(Expr::Alt(branches))
        }
    }

    fn parse_seq(&mut self) -> Result<Expr, Diagnostic> {
        let mut items = Vec::new();
        while self.starts_atom() {
            items.push(self.parse_postfix()?);
        }
        if items.is_empty() {
            return Err(Diagnostic {
                message: format!(
                    "expected a name, string, or `(` here, found {}",
                    describe(&self.peek().kind)
                ),
                span: self.peek().span,
                note: None,
            });
        }
        if items.len() == 1 {
            Ok(items.into_iter().next().unwrap())
        } else {
            Ok(Expr::Seq(items))
        }
    }

    fn starts_atom(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Ident(_) | TokenKind::Str(_) | TokenKind::LParen
        )
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let atom = self.parse_atom()?;
        if self.peek().kind == TokenKind::Question {
            self.bump();
            Ok(Expr::Opt(Box::new(atom)))
        } else {
            Ok(atom)
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, Diagnostic> {
        match self.peek().kind.clone() {
            TokenKind::Str(s) => {
                let span = self.peek().span;
                self.bump();
                Ok(Expr::Literal(s, span))
            }
            TokenKind::Ident(name) => {
                let span = self.peek().span;
                self.bump();
                Ok(Expr::Ref(name, span))
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.parse_alt()?;
                self.expect(TokenKind::RParen, "`)`")?;
                Ok(inner)
            }
            _ => Err(Diagnostic {
                message: format!(
                    "expected a name, string, or `(`, found {}",
                    describe(&self.peek().kind)
                ),
                span: self.peek().span,
                note: None,
            }),
        }
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(s) => format!("identifier `{}`", s),
        TokenKind::Str(s) => format!("string \"{}\"", s),
        TokenKind::Equals => "`=`".to_string(),
        TokenKind::Semicolon => "`;`".to_string(),
        TokenKind::Pipe => "`|`".to_string(),
        TokenKind::LParen => "`(`".to_string(),
        TokenKind::RParen => "`)`".to_string(),
        TokenKind::Question => "`?`".to_string(),
        TokenKind::Eof => "end of file".to_string(),
    }
}

/// Checks the things the grammar-level parser cannot: that every rule
/// name is unique, that every reference points at a rule that actually
/// exists, and that the file has the one rule the (future) generator
/// needs as its starting point.
pub fn validate(grammar: &Grammar) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<&str, Span> = HashMap::new();

    for rule in &grammar.rules {
        if let Some(first_span) = seen.get(rule.name.as_str()) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "rule `{}` is defined more than once (first defined at line {})",
                    rule.name, first_span.line
                ),
                span: rule.name_span,
                note: None,
            });
        } else {
            seen.insert(rule.name.as_str(), rule.name_span);
        }
    }

    let known: HashSet<&str> = grammar.rules.iter().map(|r| r.name.as_str()).collect();
    for rule in &grammar.rules {
        check_refs(&rule.expr, &known, &mut diagnostics);
    }

    if !known.contains("name") {
        diagnostics.push(Diagnostic {
            message: "grammar has no `name` rule, which is required as the entry point".to_string(),
            span: Span { line: 1, col: 1, len: 1 },
            note: Some("add a rule like `name = ...;`".to_string()),
        });
    }

    diagnostics
}

fn check_refs(expr: &Expr, known: &HashSet<&str>, out: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Literal(_, _) => {}
        Expr::Ref(name, span) => {
            if !known.contains(name.as_str()) {
                out.push(Diagnostic {
                    message: format!("undefined rule `{}`", name),
                    span: *span,
                    note: Some("not found in this file".to_string()),
                });
            }
        }
        Expr::Seq(items) | Expr::Alt(items) => {
            for item in items {
                check_refs(item, known, out);
            }
        }
        Expr::Opt(inner) => check_refs(inner, known, out),
    }
}
