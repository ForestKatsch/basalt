/// Basalt Compiler Errors - Structured error types with source locations.
use crate::ast::Span;

/// A compiler diagnostic with source location and context.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
    /// Additional context notes (e.g., "declared here", "expected because of this")
    pub notes: Vec<Note>,
}

/// A secondary annotation attached to an error.
#[derive(Debug, Clone)]
pub struct Note {
    pub message: String,
    pub span: Span,
}

impl CompileError {
    /// Create an error at a specific source location.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        CompileError {
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    /// Create an error with no source location (e.g., file I/O errors).
    pub fn bare(message: impl Into<String>) -> Self {
        CompileError {
            message: message.into(),
            span: Span::default(),
            notes: Vec::new(),
        }
    }

    /// Add a contextual note pointing at another location.
    pub fn with_note(mut self, message: impl Into<String>, span: Span) -> Self {
        self.notes.push(Note {
            message: message.into(),
            span,
        });
        self
    }

    /// Set the span if it hasn't been set yet (line == 0 means unset).
    pub fn with_span(mut self, span: Span) -> Self {
        if self.span.line == 0 {
            self.span = span;
        }
        self
    }

    /// Render this error as a human-readable diagnostic.
    ///
    /// `source` is the full source code of the file that produced this error.
    /// `filename` is the display name of the file (e.g., "example.bas").
    pub fn render(&self, source: &str, filename: &str) -> String {
        let mut out = String::new();
        let lines: Vec<&str> = source.lines().collect();

        // Header: error[filename:line:col]: message
        if self.span.line > 0 {
            out.push_str(&format!(
                "error[{}:{}:{}]: {}\n",
                filename, self.span.line, self.span.col, self.message
            ));
        } else {
            out.push_str(&format!("error[{}]: {}\n", filename, self.message));
        }

        // Source context with caret
        if self.span.line > 0 {
            render_source_context(&mut out, &lines, self.span);
        }

        // Notes
        for note in &self.notes {
            if note.span.line > 0 {
                out.push_str(&format!(
                    "  note[{}:{}]: {}\n",
                    note.span.line, note.span.col, note.message
                ));
                render_source_context(&mut out, &lines, note.span);
            } else {
                out.push_str(&format!("  note: {}\n", note.message));
            }
        }

        out
    }
}

/// Render a source code snippet with a caret pointing at the span.
fn render_source_context(out: &mut String, lines: &[&str], span: Span) {
    let line_idx = span.line as usize;
    if line_idx == 0 || line_idx > lines.len() {
        return;
    }
    let line_content = lines[line_idx - 1];
    let line_num = format!("{}", span.line);
    let gutter_width = line_num.len();

    // Line number gutter + source line
    out.push_str(&format!(
        " {:>width$} | {}\n",
        line_num,
        line_content,
        width = gutter_width
    ));

    // Caret line
    if span.col > 0 {
        let col_idx = (span.col as usize).saturating_sub(1);
        out.push_str(&format!(
            " {:>width$} | {}^\n",
            "",
            " ".repeat(col_idx),
            width = gutter_width
        ));
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.span.line > 0 {
            write!(
                f,
                "[line {}:{}] {}",
                self.span.line, self.span.col, self.message
            )
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl From<CompileError> for String {
    fn from(e: CompileError) -> String {
        e.to_string()
    }
}

/// A collection of compile errors. The compiler continues after recoverable errors
/// to report as many issues as possible in a single pass.
#[derive(Debug, Clone)]
pub struct CompileErrors {
    pub errors: Vec<CompileError>,
}

impl CompileErrors {
    pub fn new(errors: Vec<CompileError>) -> Self {
        CompileErrors { errors }
    }

    pub fn single(error: CompileError) -> Self {
        CompileErrors {
            errors: vec![error],
        }
    }

    pub fn render_all(&self, source: &str, filename: &str) -> String {
        self.errors
            .iter()
            .map(|e| e.render(source, filename))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl std::fmt::Display for CompileErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, e) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

impl From<CompileErrors> for String {
    fn from(e: CompileErrors) -> String {
        e.to_string()
    }
}

impl From<CompileError> for CompileErrors {
    fn from(e: CompileError) -> Self {
        CompileErrors::single(e)
    }
}
