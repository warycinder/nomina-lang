use crate::error::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Str(String),
    Equals,
    Semicolon,
    Pipe,
    LParen,
    RParen,
    Question,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// A `#` comment. `text` is everything after the `#` with trailing
/// whitespace removed. `trailing` is true when a token precedes the
/// comment on the same line, which the pretty printer needs to tell a
/// note on a rule from a comment sitting on its own line.
#[derive(Debug, Clone)]
pub struct Comment {
    pub line: usize,
    pub text: String,
    pub trailing: bool,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    last_token_line: usize,
    comments: Vec<Comment>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            last_token_line: 0,
            comments: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('#') => {
                    let line = self.line;
                    let trailing = self.last_token_line == line;
                    self.bump();
                    let mut text = String::new();
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        text.push(c);
                        self.bump();
                    }
                    self.comments.push(Comment {
                        line,
                        text: text.trim_end().to_string(),
                        trailing,
                    });
                }
                _ => break,
            }
        }
    }

    pub fn tokenize(mut self) -> Result<(Vec<Token>, Vec<Comment>), Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            let start_line = self.line;
            let start_col = self.col;

            let c = match self.peek() {
                Some(c) => c,
                None => {
                    tokens.push(Token {
                        kind: TokenKind::Eof,
                        span: Span { line: start_line, col: start_col, len: 1 },
                    });
                    break;
                }
            };

            let kind = match c {
                '=' => { self.bump(); TokenKind::Equals }
                ';' => { self.bump(); TokenKind::Semicolon }
                '|' => { self.bump(); TokenKind::Pipe }
                '(' => { self.bump(); TokenKind::LParen }
                ')' => { self.bump(); TokenKind::RParen }
                '?' => { self.bump(); TokenKind::Question }
                '"' => {
                    self.bump();
                    let mut s = String::new();
                    loop {
                        match self.peek() {
                            Some('"') => {
                                self.bump();
                                break;
                            }
                            Some('\n') | None => {
                                return Err(Diagnostic {
                                    message: "unterminated string literal".to_string(),
                                    span: Span { line: start_line, col: start_col, len: 1 },
                                    note: Some("strings must be closed on the same line".to_string()),
                                });
                            }
                            Some(ch) => {
                                s.push(ch);
                                self.bump();
                            }
                        }
                    }
                    TokenKind::Str(s)
                }
                c if c.is_alphabetic() || c == '_' => {
                    let mut s = String::new();
                    while let Some(ch) = self.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            s.push(ch);
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    TokenKind::Ident(s)
                }
                other => {
                    return Err(Diagnostic {
                        message: format!("unexpected character `{}`", other),
                        span: Span { line: start_line, col: start_col, len: 1 },
                        note: None,
                    });
                }
            };

            let len = match &kind {
                TokenKind::Ident(s) => s.chars().count(),
                TokenKind::Str(s) => s.chars().count() + 2,
                _ => 1,
            };
            tokens.push(Token {
                kind,
                span: Span { line: start_line, col: start_col, len },
            });
            self.last_token_line = self.line;
        }
        Ok((tokens, self.comments))
    }
}
