/// A position in a source file, plus how many characters the offending
/// token or span covers. Kept 1-based (line 1, column 1) since that is
/// what every text editor shows a human.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub note: Option<String>,
}

impl Diagnostic {
    /// Renders a rustc-style block: the message, a `-->` location line,
    /// the offending source line, and a caret underline. This is the
    /// whole point of the project, so it gets its own well-tested corner
    /// rather than being inlined at every call site.
    pub fn render(&self, filename: &str, source: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!("error: {}\n", self.message));
        out.push_str(&format!(
            " --> {}:{}:{}\n",
            filename, self.span.line, self.span.col
        ));

        let line_text = source.lines().nth(self.span.line.saturating_sub(1)).unwrap_or("");
        let line_num = self.span.line.to_string();
        let gutter = " ".repeat(line_num.len());

        out.push_str(&format!("{} |\n", gutter));
        out.push_str(&format!("{} | {}\n", line_num, line_text));

        let caret_pad = " ".repeat(self.span.col.saturating_sub(1));
        let carets = "^".repeat(self.span.len.max(1));
        out.push_str(&format!("{} | {}{}", gutter, caret_pad, carets));
        if let Some(note) = &self.note {
            out.push(' ');
            out.push_str(note);
        }
        out.push('\n');
        out
    }
}
