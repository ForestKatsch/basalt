/// Table-driven syntax highlighter for fenced code blocks.
///
/// Produces `<span class="XX">…</span>` tokens where the class names are:
///   kw = keyword, ty = type, st = string, nu = number,
///   cm = comment, fn = function call, op = operator.
/// Identifiers, whitespace, and punctuation get no wrapper.

#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenKind {
    Keyword,
    Type,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Function,
    Identifier,
    Whitespace,
}

struct Token {
    kind: TokenKind,
    text: String,
}

struct LangDef {
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    line_comment: &'static str,
    string_delimiters: &'static [char],
}

const BASALT: LangDef = LangDef {
    keywords: &[
        "fn", "let", "mut", "return", "if", "else", "match", "for", "in", "while", "loop", "break",
        "continue", "type", "guard", "import", "as", "true", "false", "nil", "is",
    ],
    types: &[
        "i8",
        "i16",
        "i32",
        "i64",
        "u8",
        "u16",
        "u32",
        "u64",
        "f64",
        "bool",
        "string",
        "nil",
        "Stdout",
        "Stdin",
        "Fs",
        "Env",
        "Highlight",
        "Self",
    ],
    line_comment: "//",
    string_delimiters: &['"'],
};

const TYPESCRIPT: LangDef = LangDef {
    keywords: &[
        "function",
        "const",
        "let",
        "var",
        "return",
        "if",
        "else",
        "for",
        "while",
        "do",
        "switch",
        "case",
        "break",
        "continue",
        "class",
        "extends",
        "implements",
        "interface",
        "type",
        "enum",
        "import",
        "export",
        "from",
        "default",
        "async",
        "await",
        "new",
        "this",
        "super",
        "try",
        "catch",
        "finally",
        "throw",
        "typeof",
        "instanceof",
        "in",
        "of",
        "true",
        "false",
        "null",
        "undefined",
        "void",
        "yield",
        "delete",
    ],
    types: &[
        "string", "number", "boolean", "any", "void", "never", "unknown", "object", "symbol",
        "bigint", "Array", "Map", "Set", "Promise", "Record", "Partial", "Required", "Readonly",
    ],
    line_comment: "//",
    string_delimiters: &['"', '\'', '`'],
};

const RUST: LangDef = LangDef {
    keywords: &[
        "fn", "let", "mut", "const", "static", "return", "if", "else", "match", "for", "in",
        "while", "loop", "break", "continue", "struct", "enum", "impl", "trait", "type", "use",
        "mod", "pub", "crate", "self", "super", "as", "where", "async", "await", "move", "ref",
        "true", "false", "unsafe", "extern", "dyn",
    ],
    types: &[
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64", "bool", "char", "str", "String", "Vec", "Box", "Rc", "Arc", "Option",
        "Result", "Self", "HashMap", "HashSet", "PathBuf", "Path",
    ],
    line_comment: "//",
    string_delimiters: &['"'],
};

const SHELL: LangDef = LangDef {
    keywords: &[
        "if", "then", "else", "fi", "for", "do", "done", "while", "case", "esac", "in", "function",
        "return", "exit", "echo", "cd", "ls", "rm", "cp", "mv", "mkdir", "cat", "grep", "sed",
        "awk", "export", "source", "sudo",
    ],
    types: &[],
    line_comment: "#",
    string_delimiters: &['"', '\''],
};

fn tokenize(source: &str, lang: &LangDef) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        // Whitespace
        if chars[i].is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Whitespace,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Line comment (check byte offset via char position)
        if !lang.line_comment.is_empty()
            && source[char_byte_offset(source, i)..].starts_with(lang.line_comment)
        {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // String literals
        if lang.string_delimiters.contains(&chars[i]) {
            let delim = chars[i];
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != delim {
                if chars[i] == '\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1; // consume closing delimiter
            }
            tokens.push(Token {
                kind: TokenKind::String,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Numbers
        if chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if chars[i] == '0'
                && i + 1 < chars.len()
                && (chars[i + 1] == 'x' || chars[i + 1] == 'b')
            {
                i += 2;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
            } else {
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == 'e'
                        || chars[i] == 'E'
                        || chars[i] == '_')
                {
                    i += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }

        // Identifier / keyword / type / function call
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            // Peek ahead past whitespace for '(' to detect function calls
            let next_non_ws = chars[i..].iter().find(|c| !c.is_whitespace()).copied();
            let kind = if lang.keywords.contains(&word.as_str()) {
                TokenKind::Keyword
            } else if lang.types.contains(&word.as_str()) {
                TokenKind::Type
            } else if word.starts_with(|c: char| c.is_uppercase()) {
                // PascalCase identifiers treated as types even if not in the table
                TokenKind::Type
            } else if next_non_ws == Some('(') {
                TokenKind::Function
            } else {
                TokenKind::Identifier
            };
            tokens.push(Token { kind, text: word });
            continue;
        }

        // Operator / punctuation — try two-char first
        let ch = chars[i];
        i += 1;
        let mut text = ch.to_string();
        if i < chars.len() {
            let pair: String = [ch, chars[i]].iter().collect();
            if matches!(
                pair.as_str(),
                "=>" | "->"
                    | "=="
                    | "!="
                    | "<="
                    | ">="
                    | "&&"
                    | "||"
                    | "<<"
                    | ">>"
                    | "**"
                    | "::"
                    | "?."
            ) {
                text = pair;
                i += 1;
            }
        }
        let kind = match ch {
            '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '^' | '~' | '?'
            | ':' => TokenKind::Operator,
            '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | '.' | '@' | '#' => {
                TokenKind::Punctuation
            }
            _ => TokenKind::Identifier,
        };
        tokens.push(Token { kind, text });
    }

    tokens
}

/// Byte offset of the `n`th char in a UTF-8 string.
fn char_byte_offset(s: &str, n: usize) -> usize {
    s.char_indices()
        .nth(n)
        .map(|(offset, _)| offset)
        .unwrap_or(s.len())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn token_class(kind: TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Keyword => Some("kw"),
        TokenKind::Type => Some("ty"),
        TokenKind::String => Some("st"),
        TokenKind::Number => Some("nu"),
        TokenKind::Comment => Some("cm"),
        TokenKind::Function => Some("fn"),
        TokenKind::Operator => Some("op"),
        TokenKind::Punctuation | TokenKind::Identifier | TokenKind::Whitespace => None,
    }
}

fn lang_def(name: &str) -> Option<&'static LangDef> {
    match name {
        "basalt" | "bas" => Some(&BASALT),
        "typescript" | "ts" => Some(&TYPESCRIPT),
        "javascript" | "js" => Some(&TYPESCRIPT),
        "rust" | "rs" => Some(&RUST),
        "sh" | "bash" | "shell" | "zsh" => Some(&SHELL),
        _ => None,
    }
}

/// Highlight source code, returning HTML with `<span class="XX">` wrappers.
pub fn highlight_code(source: &str, lang_name: &str) -> String {
    let lang = match lang_def(lang_name) {
        Some(l) => l,
        None => return html_escape(source),
    };
    let tokens = tokenize(source, lang);
    let mut out = String::with_capacity(source.len() * 2);
    for tok in &tokens {
        let escaped = html_escape(&tok.text);
        match token_class(tok.kind) {
            Some(cls) => {
                out.push_str("<span class=\"");
                out.push_str(cls);
                out.push_str("\">");
                out.push_str(&escaped);
                out.push_str("</span>");
            }
            None => out.push_str(&escaped),
        }
    }
    out
}

/// Highlight for inline code — same as block for now.
pub fn highlight_inline(source: &str, lang_name: &str) -> String {
    highlight_code(source, lang_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basalt_fn_keyword() {
        let result = highlight_code("fn main() { return 42 }", "basalt");
        assert!(result.contains("<span class=\"kw\">fn</span>"));
        assert!(result.contains("<span class=\"fn\">main</span>"));
        assert!(result.contains("<span class=\"kw\">return</span>"));
        assert!(result.contains("<span class=\"nu\">42</span>"));
    }

    #[test]
    fn rust_types_and_strings() {
        let result = highlight_code("let s: String = \"hello\"", "rust");
        assert!(result.contains("<span class=\"kw\">let</span>"));
        assert!(result.contains("<span class=\"ty\">String</span>"));
        assert!(result.contains("<span class=\"st\">\"hello\"</span>"));
    }

    #[test]
    fn typescript_keywords() {
        let result = highlight_code("const x = await fetch(url)", "ts");
        assert!(result.contains("<span class=\"kw\">const</span>"));
        assert!(result.contains("<span class=\"kw\">await</span>"));
        assert!(result.contains("<span class=\"fn\">fetch</span>"));
    }

    #[test]
    fn unknown_lang_escapes() {
        let result = highlight_code("<script>alert('xss')</script>", "unknown");
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn comments_highlighted() {
        let result = highlight_code("// a comment\nlet x = 1", "basalt");
        assert!(result.contains("<span class=\"cm\">// a comment</span>"));
    }

    #[test]
    fn inline_same_as_code() {
        let src = "fn test()";
        assert_eq!(
            highlight_code(src, "basalt"),
            highlight_inline(src, "basalt")
        );
    }
}
