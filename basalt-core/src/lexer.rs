/// Basalt Lexer - Converts source text into tokens.
use crate::ast::Span;
use crate::error::CompileError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    IntLit(i128),
    FloatLit(f64),
    StringLit(String),
    /// String with interpolations: parts alternate between literal strings and expressions
    /// e.g. "Hello, \(name)!" => InterpolatedString([StringPart("Hello, "), ExprPart("name"), StringPart("!")])
    InterpolatedString(Vec<StringPart>),
    True,
    False,
    Nil,

    // Identifiers
    Ident(String),     // lowercase start
    TypeIdent(String), // uppercase start

    // Keywords
    Let,
    Mut,
    Fn,
    Return,
    If,
    Else,
    Match,
    For,
    In,
    While,
    Loop,
    Break,
    Continue,
    Type,
    Guard,
    Import,
    As,
    Is,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,   // **
    Ampersand,  // &
    Pipe,       // |
    Caret,      // ^
    Tilde,      // ~
    ShiftLeft,  // <<
    ShiftRight, // >>
    AmpAmp,     // &&
    PipePipe,   // ||
    Bang,       // !
    Eq,         // =
    EqEq,       // ==
    BangEq,     // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    DotDot,     // ..
    Arrow,      // ->
    FatArrow,   // =>
    Question,   // ?

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Punctuation
    Dot,
    Comma,
    Colon,

    // Special
    Newline,
    Eof,
}

impl Token {
    /// Human-readable name for use in error messages.
    pub fn display_name(&self) -> &'static str {
        match self {
            Token::LParen => "(",
            Token::RParen => ")",
            Token::LBrace => "{",
            Token::RBrace => "}",
            Token::LBracket => "[",
            Token::RBracket => "]",
            Token::Comma => ",",
            Token::Colon => ":",
            Token::Dot => ".",
            Token::Eq => "=",
            Token::Arrow => "->",
            Token::FatArrow => "=>",
            Token::Newline => "newline",
            Token::Eof => "end of file",
            Token::Let => "let",
            Token::Mut => "mut",
            Token::Fn => "fn",
            Token::Return => "return",
            Token::If => "if",
            Token::Else => "else",
            Token::Match => "match",
            Token::For => "for",
            Token::In => "in",
            Token::While => "while",
            Token::Loop => "loop",
            Token::Break => "break",
            Token::Continue => "continue",
            Token::Type => "type",
            Token::Guard => "guard",
            Token::Import => "import",
            Token::As => "as",
            Token::Is => "is",
            Token::True => "true",
            Token::False => "false",
            Token::Nil => "nil",
            _ => "token",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),
    Expr(Vec<Token>),
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

pub fn lex(source: &str) -> Result<Vec<SpannedToken>, CompileError> {
    let mut lexer = Lexer::new(source);
    lexer.lex_all()
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn make_token(&self, token: Token, line: usize, col: usize) -> SpannedToken {
        SpannedToken { token, line, col }
    }

    fn lex_all(&mut self) -> Result<Vec<SpannedToken>, CompileError> {
        let mut tokens = Vec::new();
        let mut last_was_newline = true; // Start as if we just had a newline

        loop {
            self.skip_whitespace_not_newline();

            // Skip comments
            if self.peek() == Some('/') && self.peek_at(1) == Some('/') {
                self.skip_line_comment();
                continue;
            }

            let line = self.line;
            let col = self.col;

            match self.peek() {
                None => {
                    // Ensure there's a final newline before EOF
                    if !last_was_newline {
                        tokens.push(self.make_token(Token::Newline, line, col));
                    }
                    tokens.push(self.make_token(Token::Eof, line, col));
                    break;
                }
                Some('\n') => {
                    self.advance();
                    if !last_was_newline
                        && !Self::continues_on_next_line(tokens.last())
                        && !self.next_line_continues()
                    {
                        tokens.push(self.make_token(Token::Newline, line, col));
                        last_was_newline = true;
                    }
                    continue;
                }
                Some('\r') => {
                    self.advance();
                    continue;
                }
                Some('\\') if self.peek_at(1) == Some('\\') => {
                    // Multiline string
                    let tok = self.lex_multiline_string()?;
                    tokens.push(self.make_token(tok, line, col));
                    // The multiline string consumed through newlines, so emit
                    // a newline token to terminate the statement.
                    tokens.push(self.make_token(Token::Newline, self.line, self.col));
                    last_was_newline = true;
                    continue;
                }
                Some(ch) => {
                    let tok = self.lex_token(ch)?;
                    last_was_newline = tok.token == Token::Newline;
                    tokens.push(tok);
                }
            }
        }

        Ok(tokens)
    }

    /// Returns true if the given token at end-of-line implies the expression
    /// continues on the next line (suppress the newline token).
    fn continues_on_next_line(last: Option<&SpannedToken>) -> bool {
        match last {
            None => false,
            Some(st) => matches!(
                st.token,
                // Binary operators
                Token::Plus
                    | Token::Minus
                    | Token::Star
                    | Token::Slash
                    | Token::Percent
                    | Token::StarStar
                    | Token::Ampersand
                    | Token::Pipe
                    | Token::Caret
                    | Token::Tilde
                    | Token::ShiftLeft
                    | Token::ShiftRight
                    | Token::AmpAmp
                    | Token::PipePipe
                    | Token::EqEq
                    | Token::BangEq
                    | Token::Lt
                    | Token::LtEq
                    | Token::Gt
                    | Token::GtEq
                    | Token::DotDot
                    // Assignment and arrows
                    | Token::Eq
                    | Token::Arrow
                    | Token::FatArrow
                    // Punctuation that implies continuation
                    | Token::Comma
                    | Token::Dot
                    | Token::Colon
                    // Open brackets
                    | Token::LParen
                    | Token::LBracket
                    | Token::LBrace
            ),
        }
    }

    /// Peek ahead (without consuming) to check if the next non-whitespace
    /// character starts a continuation token (like `.` for method chaining).
    fn next_line_continues(&self) -> bool {
        let mut i = self.pos;
        // Skip whitespace and newlines
        while i < self.chars.len() {
            let ch = self.chars[i];
            if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
                i += 1;
            } else {
                break;
            }
        }
        if i >= self.chars.len() {
            return false;
        }
        // Check if the next non-whitespace char is a continuation prefix
        matches!(self.chars[i], '.')
    }

    fn skip_whitespace_not_newline(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn lex_token(&mut self, ch: char) -> Result<SpannedToken, CompileError> {
        let line = self.line;
        let col = self.col;

        match ch {
            '"' => {
                let tok = self.lex_string()?;
                Ok(self.make_token(tok, line, col))
            }
            '0'..='9' => {
                let tok = self.lex_number()?;
                Ok(self.make_token(tok, line, col))
            }
            'a'..='z' | '_' => {
                let tok = self.lex_identifier()?;
                Ok(self.make_token(tok, line, col))
            }
            'A'..='Z' => {
                let tok = self.lex_type_identifier()?;
                Ok(self.make_token(tok, line, col))
            }
            '+' => {
                self.advance();
                Ok(self.make_token(Token::Plus, line, col))
            }
            '-' => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(self.make_token(Token::Arrow, line, col))
                } else {
                    Ok(self.make_token(Token::Minus, line, col))
                }
            }
            '*' => {
                self.advance();
                if self.peek() == Some('*') {
                    self.advance();
                    Ok(self.make_token(Token::StarStar, line, col))
                } else {
                    Ok(self.make_token(Token::Star, line, col))
                }
            }
            '/' => {
                self.advance();
                Ok(self.make_token(Token::Slash, line, col))
            }
            '%' => {
                self.advance();
                Ok(self.make_token(Token::Percent, line, col))
            }
            '&' => {
                self.advance();
                if self.peek() == Some('&') {
                    self.advance();
                    Ok(self.make_token(Token::AmpAmp, line, col))
                } else {
                    Ok(self.make_token(Token::Ampersand, line, col))
                }
            }
            '|' => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(self.make_token(Token::PipePipe, line, col))
                } else {
                    Ok(self.make_token(Token::Pipe, line, col))
                }
            }
            '^' => {
                self.advance();
                Ok(self.make_token(Token::Caret, line, col))
            }
            '~' => {
                self.advance();
                Ok(self.make_token(Token::Tilde, line, col))
            }
            '!' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(Token::BangEq, line, col))
                } else {
                    Ok(self.make_token(Token::Bang, line, col))
                }
            }
            '=' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(Token::EqEq, line, col))
                } else if self.peek() == Some('>') {
                    self.advance();
                    Ok(self.make_token(Token::FatArrow, line, col))
                } else {
                    Ok(self.make_token(Token::Eq, line, col))
                }
            }
            '<' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(Token::LtEq, line, col))
                } else if self.peek() == Some('<') {
                    self.advance();
                    Ok(self.make_token(Token::ShiftLeft, line, col))
                } else {
                    Ok(self.make_token(Token::Lt, line, col))
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(self.make_token(Token::GtEq, line, col))
                } else if self.peek() == Some('>') {
                    self.advance();
                    Ok(self.make_token(Token::ShiftRight, line, col))
                } else {
                    Ok(self.make_token(Token::Gt, line, col))
                }
            }
            '?' => {
                self.advance();
                Ok(self.make_token(Token::Question, line, col))
            }
            '(' => {
                self.advance();
                Ok(self.make_token(Token::LParen, line, col))
            }
            ')' => {
                self.advance();
                Ok(self.make_token(Token::RParen, line, col))
            }
            '{' => {
                self.advance();
                Ok(self.make_token(Token::LBrace, line, col))
            }
            '}' => {
                self.advance();
                Ok(self.make_token(Token::RBrace, line, col))
            }
            '[' => {
                self.advance();
                Ok(self.make_token(Token::LBracket, line, col))
            }
            ']' => {
                self.advance();
                Ok(self.make_token(Token::RBracket, line, col))
            }
            '.' => {
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    Ok(self.make_token(Token::DotDot, line, col))
                } else {
                    Ok(self.make_token(Token::Dot, line, col))
                }
            }
            ',' => {
                self.advance();
                Ok(self.make_token(Token::Comma, line, col))
            }
            ':' => {
                self.advance();
                Ok(self.make_token(Token::Colon, line, col))
            }
            _ => Err(CompileError::new(
                format!("unexpected character '{}'", ch),
                Span::new(line as u32, col as u32),
            )),
        }
    }

    fn lex_number(&mut self) -> Result<Token, CompileError> {
        let start = self.pos;

        // Check for hex/binary prefix
        if self.peek() == Some('0') {
            if self.peek_at(1) == Some('x') || self.peek_at(1) == Some('X') {
                self.advance(); // 0
                self.advance(); // x
                let hex_start = self.pos;
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_hexdigit() || ch == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if self.pos == hex_start {
                    return Err(CompileError::new(
                        "expected hex digit",
                        Span::new(self.line as u32, self.col as u32),
                    ));
                }
                let hex_str: String = self.chars[hex_start..self.pos]
                    .iter()
                    .filter(|c| **c != '_')
                    .collect();
                let val = i128::from_str_radix(&hex_str, 16).map_err(|e| {
                    CompileError::new(
                        format!("invalid hex literal: {}", e),
                        Span::new(self.line as u32, self.col as u32),
                    )
                })?;
                return Ok(Token::IntLit(val));
            }
            if self.peek_at(1) == Some('b') || self.peek_at(1) == Some('B') {
                self.advance(); // 0
                self.advance(); // b
                let bin_start = self.pos;
                while let Some(ch) = self.peek() {
                    if ch == '0' || ch == '1' || ch == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if self.pos == bin_start {
                    return Err(CompileError::new(
                        "expected binary digit",
                        Span::new(self.line as u32, self.col as u32),
                    ));
                }
                let bin_str: String = self.chars[bin_start..self.pos]
                    .iter()
                    .filter(|c| **c != '_')
                    .collect();
                let val = i128::from_str_radix(&bin_str, 2).map_err(|e| {
                    CompileError::new(
                        format!("invalid binary literal: {}", e),
                        Span::new(self.line as u32, self.col as u32),
                    )
                })?;
                return Ok(Token::IntLit(val));
            }
        }

        // Decimal digits (with optional _ separators)
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        // Check for float
        let is_float = if self.peek() == Some('.') && self.peek_at(1) != Some('.') {
            // Look ahead to see if there's a digit after the dot
            if let Some(ch) = self.peek_at(1) {
                if ch.is_ascii_digit() {
                    self.advance(); // consume '.'
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_digit() || ch == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // Check for exponent
        let has_exp = if self.peek() == Some('e') || self.peek() == Some('E') {
            self.advance();
            if self.peek() == Some('+') || self.peek() == Some('-') {
                self.advance();
            }
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            true
        } else {
            false
        };

        let num_str: String = self.chars[start..self.pos]
            .iter()
            .filter(|c| **c != '_')
            .collect();

        if is_float || has_exp {
            let val: f64 = num_str.parse().map_err(|e| {
                CompileError::new(
                    format!("invalid float literal '{}': {}", num_str, e),
                    Span::new(self.line as u32, self.col as u32),
                )
            })?;
            Ok(Token::FloatLit(val))
        } else {
            let val: i128 = num_str.parse().map_err(|e| {
                CompileError::new(
                    format!("invalid integer literal '{}': {}", num_str, e),
                    Span::new(self.line as u32, self.col as u32),
                )
            })?;
            Ok(Token::IntLit(val))
        }
    }

    fn lex_string(&mut self) -> Result<Token, CompileError> {
        self.advance(); // consume opening quote
        let mut parts: Vec<StringPart> = Vec::new();
        let mut current = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(CompileError::new(
                        "unterminated string",
                        Span::new(self.line as u32, self.col as u32),
                    ))
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('n') => {
                            self.advance();
                            current.push('\n');
                        }
                        Some('t') => {
                            self.advance();
                            current.push('\t');
                        }
                        Some('r') => {
                            self.advance();
                            current.push('\r');
                        }
                        Some('\\') => {
                            self.advance();
                            current.push('\\');
                        }
                        Some('"') => {
                            self.advance();
                            current.push('"');
                        }
                        Some('0') => {
                            self.advance();
                            current.push('\0');
                        }
                        Some('e') => {
                            self.advance();
                            current.push('\x1B');
                        }
                        Some('(') => {
                            // String interpolation
                            self.advance(); // consume '('
                            if !current.is_empty() {
                                parts.push(StringPart::Literal(std::mem::take(&mut current)));
                            }
                            let expr_tokens = self.lex_interpolation_expr()?;
                            parts.push(StringPart::Expr(expr_tokens));
                        }
                        Some(ch) => {
                            return Err(CompileError::new(
                                format!("unknown escape sequence '\\{}'", ch),
                                Span::new(self.line as u32, self.col as u32),
                            ))
                        }
                        None => {
                            return Err(CompileError::new(
                                "unterminated escape",
                                Span::new(self.line as u32, self.col as u32),
                            ))
                        }
                    }
                }
                Some(ch) => {
                    self.advance();
                    current.push(ch);
                }
            }
        }

        if parts.is_empty() {
            Ok(Token::StringLit(current))
        } else {
            if !current.is_empty() {
                parts.push(StringPart::Literal(current));
            }
            Ok(Token::InterpolatedString(parts))
        }
    }

    fn lex_interpolation_expr(&mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        let mut paren_depth = 1;

        loop {
            self.skip_whitespace_not_newline();
            match self.peek() {
                None => {
                    return Err(CompileError::new(
                        "unterminated string interpolation",
                        Span::new(self.line as u32, self.col as u32),
                    ))
                }
                Some('(') => {
                    paren_depth += 1;
                    let line = self.line;
                    let col = self.col;
                    self.advance();
                    tokens.push(SpannedToken {
                        token: Token::LParen,
                        line,
                        col,
                    });
                }
                Some(')') => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        self.advance();
                        break;
                    }
                    let line = self.line;
                    let col = self.col;
                    self.advance();
                    tokens.push(SpannedToken {
                        token: Token::RParen,
                        line,
                        col,
                    });
                }
                Some(ch) => {
                    let tok = self.lex_token(ch)?;
                    tokens.push(tok);
                }
            }
        }

        Ok(tokens.into_iter().map(|st| st.token).collect())
    }

    fn lex_multiline_string(&mut self) -> Result<Token, CompileError> {
        let mut lines = Vec::new();

        loop {
            // We expect to be at the start of a \\ line (after whitespace)
            if self.peek() == Some('\\') && self.peek_at(1) == Some('\\') {
                self.advance(); // first backslash
                self.advance(); // second backslash
                let mut line_content = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        self.advance();
                        break;
                    }
                    if ch == '\r' {
                        self.advance();
                        if self.peek() == Some('\n') {
                            self.advance();
                        }
                        break;
                    }
                    self.advance();
                    line_content.push(ch);
                }
                lines.push(line_content);

                // Skip whitespace before potential next multiline continuation
                self.skip_whitespace_not_newline();
                // Check if next line continues with \\
                if !(self.peek() == Some('\\') && self.peek_at(1) == Some('\\')) {
                    break;
                }
            } else {
                break;
            }
        }

        // Join with newlines. Last line does NOT get a trailing newline.
        let result = lines.join("\n");
        Ok(Token::StringLit(result))
    }

    fn lex_identifier(&mut self) -> Result<Token, CompileError> {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let ident: String = self.chars[start..self.pos].iter().collect();

        let token = match ident.as_str() {
            "let" => Token::Let,
            "mut" => Token::Mut,
            "fn" => Token::Fn,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "match" => Token::Match,
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "loop" => Token::Loop,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "type" => Token::Type,
            "guard" => Token::Guard,
            "import" => Token::Import,
            "as" => Token::As,
            "is" => Token::Is,
            "true" => Token::True,
            "false" => Token::False,
            "nil" => Token::Nil,
            "async" | "await" => {
                return Err(CompileError::new(
                    format!("'{}' is reserved for future use", ident),
                    Span::new(self.line as u32, self.col as u32),
                ))
            }
            "_" => Token::Ident("_".to_string()), // wildcard
            _ => Token::Ident(ident),
        };
        Ok(token)
    }

    fn lex_type_identifier(&mut self) -> Result<Token, CompileError> {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let ident: String = self.chars[start..self.pos].iter().collect();
        // Type identifiers like Self are just TypeIdent
        Ok(Token::TypeIdent(ident))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(source: &str) -> Vec<Token> {
        let tokens = lex(source).unwrap();
        tokens.into_iter().map(|st| st.token).collect()
    }

    fn tok_no_newlines(source: &str) -> Vec<Token> {
        tok(source)
            .into_iter()
            .filter(|t| *t != Token::Newline && *t != Token::Eof)
            .collect()
    }

    #[test]
    fn test_integer_literals() {
        assert_eq!(tok_no_newlines("42"), vec![Token::IntLit(42)]);
        assert_eq!(tok_no_newlines("0xFF"), vec![Token::IntLit(255)]);
        assert_eq!(tok_no_newlines("0b1010"), vec![Token::IntLit(10)]);
        assert_eq!(tok_no_newlines("0"), vec![Token::IntLit(0)]);
    }

    #[test]
    fn test_float_literals() {
        assert_eq!(tok_no_newlines("3.14"), vec![Token::FloatLit(3.14)]);
        assert_eq!(tok_no_newlines("1.0e10"), vec![Token::FloatLit(1.0e10)]);
        assert_eq!(tok_no_newlines("2.5E-3"), vec![Token::FloatLit(2.5e-3)]);
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(
            tok_no_newlines(r#""hello""#),
            vec![Token::StringLit("hello".to_string())]
        );
        assert_eq!(
            tok_no_newlines(r#""line 1\nline 2""#),
            vec![Token::StringLit("line 1\nline 2".to_string())]
        );
        assert_eq!(
            tok_no_newlines(r#""tab\there""#),
            vec![Token::StringLit("tab\there".to_string())]
        );
    }

    #[test]
    fn test_string_interpolation() {
        let tokens = tok_no_newlines(r#""Hello, \(name)!""#);
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::InterpolatedString(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], StringPart::Literal("Hello, ".to_string()));
                assert_eq!(
                    parts[1],
                    StringPart::Expr(vec![Token::Ident("name".to_string())])
                );
                assert_eq!(parts[2], StringPart::Literal("!".to_string()));
            }
            _ => panic!("expected InterpolatedString"),
        }
    }

    #[test]
    fn test_keywords() {
        assert_eq!(tok_no_newlines("let"), vec![Token::Let]);
        assert_eq!(tok_no_newlines("mut"), vec![Token::Mut]);
        assert_eq!(tok_no_newlines("fn"), vec![Token::Fn]);
        assert_eq!(tok_no_newlines("return"), vec![Token::Return]);
        assert_eq!(tok_no_newlines("if"), vec![Token::If]);
        assert_eq!(tok_no_newlines("else"), vec![Token::Else]);
        assert_eq!(tok_no_newlines("match"), vec![Token::Match]);
        assert_eq!(tok_no_newlines("for"), vec![Token::For]);
        assert_eq!(tok_no_newlines("in"), vec![Token::In]);
        assert_eq!(tok_no_newlines("while"), vec![Token::While]);
        assert_eq!(tok_no_newlines("loop"), vec![Token::Loop]);
        assert_eq!(tok_no_newlines("break"), vec![Token::Break]);
        assert_eq!(tok_no_newlines("continue"), vec![Token::Continue]);
        assert_eq!(tok_no_newlines("type"), vec![Token::Type]);
        assert_eq!(tok_no_newlines("guard"), vec![Token::Guard]);
        assert_eq!(tok_no_newlines("import"), vec![Token::Import]);
        assert_eq!(tok_no_newlines("as"), vec![Token::As]);
        assert_eq!(tok_no_newlines("is"), vec![Token::Is]);
        assert_eq!(tok_no_newlines("true"), vec![Token::True]);
        assert_eq!(tok_no_newlines("false"), vec![Token::False]);
        assert_eq!(tok_no_newlines("nil"), vec![Token::Nil]);
    }

    #[test]
    fn test_operators() {
        assert_eq!(tok_no_newlines("+"), vec![Token::Plus]);
        assert_eq!(tok_no_newlines("**"), vec![Token::StarStar]);
        assert_eq!(tok_no_newlines("->"), vec![Token::Arrow]);
        assert_eq!(tok_no_newlines("=>"), vec![Token::FatArrow]);
        assert_eq!(tok_no_newlines("=="), vec![Token::EqEq]);
        assert_eq!(tok_no_newlines("!="), vec![Token::BangEq]);
        assert_eq!(tok_no_newlines("<="), vec![Token::LtEq]);
        assert_eq!(tok_no_newlines(">="), vec![Token::GtEq]);
        assert_eq!(tok_no_newlines("<<"), vec![Token::ShiftLeft]);
        assert_eq!(tok_no_newlines(">>"), vec![Token::ShiftRight]);
        assert_eq!(tok_no_newlines("&&"), vec![Token::AmpAmp]);
        assert_eq!(tok_no_newlines("||"), vec![Token::PipePipe]);
        assert_eq!(tok_no_newlines(".."), vec![Token::DotDot]);
    }

    #[test]
    fn test_identifiers() {
        assert_eq!(
            tok_no_newlines("foo"),
            vec![Token::Ident("foo".to_string())]
        );
        assert_eq!(
            tok_no_newlines("_bar"),
            vec![Token::Ident("_bar".to_string())]
        );
        assert_eq!(
            tok_no_newlines("Point"),
            vec![Token::TypeIdent("Point".to_string())]
        );
        assert_eq!(
            tok_no_newlines("Self"),
            vec![Token::TypeIdent("Self".to_string())]
        );
    }

    #[test]
    fn test_line_comment() {
        assert_eq!(tok_no_newlines("42 // comment"), vec![Token::IntLit(42)]);
    }

    #[test]
    fn test_complex_expression() {
        let tokens = tok_no_newlines("let x = 42 + 3");
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Ident("x".to_string()),
                Token::Eq,
                Token::IntLit(42),
                Token::Plus,
                Token::IntLit(3),
            ]
        );
    }

    #[test]
    fn test_newlines_as_separators() {
        let tokens = tok("let x = 1\nlet y = 2\n");
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Ident("x".to_string()),
                Token::Eq,
                Token::IntLit(1),
                Token::Newline,
                Token::Let,
                Token::Ident("y".to_string()),
                Token::Eq,
                Token::IntLit(2),
                Token::Newline,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_multiline_string() {
        let src = "    \\\\hello\n    \\\\world";
        let tokens = tok_no_newlines(src);
        assert_eq!(tokens, vec![Token::StringLit("hello\nworld".to_string())]);
    }
}
