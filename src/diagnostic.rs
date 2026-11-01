use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub file: String,
    pub notes: Vec<String>,
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            file: String::new(),
            notes: Vec::new(),
        }
    }
    pub fn in_file(mut self, file: impl Into<String>) -> Self {
        if self.file.is_empty() {
            self.file = file.into();
        }
        self
    }
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
    pub fn render(&self, source: Option<&str>) -> String {
        let mut out = format!(
            "{}:{}:{}\nError: {}",
            if self.file.is_empty() {
                "<source>"
            } else {
                &self.file
            },
            self.span.line.max(1),
            self.span.column.max(1),
            self.message
        );
        if let Some(line) = source.and_then(|s| s.lines().nth(self.span.line.saturating_sub(1))) {
            out.push_str(&format!(
                "\n  {line}\n  {}^",
                " ".repeat(self.span.column.saturating_sub(1).min(200))
            ));
        }
        for note in &self.notes {
            out.push_str(&format!("\nnote: {note}"));
        }
        out
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(None))
    }
}
impl std::error::Error for Diagnostic {}
