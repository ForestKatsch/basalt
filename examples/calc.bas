// Expression calculator with proper operator precedence
// Supports: +, -, *, /, parentheses, negative numbers
// Examples: "2 + 3 * 4" => 14, "(2 + 3) * 4" => 20

type Token {
    Num(f64)
    Plus
    Minus
    Star
    Slash
    LParen
    RParen
    End
}

type ParseState {
    input: string
    pos: i64
}

fn peek_char(s: ParseState) -> string {
    if s.pos >= s.input.length { return "" }
    return s.input.char_at(s.pos)
}

fn advance(s: ParseState) -> ParseState {
    return ParseState { input: s.input, pos: s.pos + 1 }
}

fn skip_spaces(s: ParseState) -> ParseState {
    let mut st = s
    while peek_char(st) == " " {
        st = advance(st)
    }
    return st
}

fn next_token(s: ParseState) -> (Token, ParseState) {
    let mut st = skip_spaces(s)
    let ch = peek_char(st)

    if ch == "" { return (Token.End, st) }
    if ch == "+" { return (Token.Plus, advance(st)) }
    if ch == "-" { return (Token.Minus, advance(st)) }
    if ch == "*" { return (Token.Star, advance(st)) }
    if ch == "/" { return (Token.Slash, advance(st)) }
    if ch == "(" { return (Token.LParen, advance(st)) }
    if ch == ")" { return (Token.RParen, advance(st)) }

    // Must be a number
    let mut num_str = ""
    let mut has_dot = false
    while true {
        let c = peek_char(st)
        if c == "." && !has_dot {
            has_dot = true
            num_str = num_str + c
            st = advance(st)
        } else if c >= "0" && c <= "9" {
            num_str = num_str + c
            st = advance(st)
        } else {
            break
        }
    }

    if num_str.length == 0 {
        panic("unexpected character: " + ch)
    }

    return (Token.Num(num_str as f64), st)
}

// Recursive descent parser
// expr     = term (('+' | '-') term)*
// term     = unary (('*' | '/') unary)*
// unary    = '-' unary | primary
// primary  = NUMBER | '(' expr ')'

fn parse_expr(s: ParseState) -> (f64, ParseState) {
    let result = parse_term(s)
    let mut val = result.0
    let mut st = result.1

    loop {
        let tok_result = next_token(st)
        let tok = tok_result.0
        let after = tok_result.1

        match tok {
            Token.Plus => {
                let rhs = parse_term(after)
                val = val + rhs.0
                st = rhs.1
            }
            Token.Minus => {
                let rhs = parse_term(after)
                val = val - rhs.0
                st = rhs.1
            }
            _ => break
        }
    }

    return (val, st)
}

fn parse_term(s: ParseState) -> (f64, ParseState) {
    let result = parse_unary(s)
    let mut val = result.0
    let mut st = result.1

    loop {
        let tok_result = next_token(st)
        let tok = tok_result.0
        let after = tok_result.1

        match tok {
            Token.Star => {
                let rhs = parse_unary(after)
                val = val * rhs.0
                st = rhs.1
            }
            Token.Slash => {
                let rhs = parse_unary(after)
                val = val / rhs.0
                st = rhs.1
            }
            _ => break
        }
    }

    return (val, st)
}

fn parse_unary(s: ParseState) -> (f64, ParseState) {
    let tok_result = next_token(s)
    match tok_result.0 {
        Token.Minus => {
            let inner = parse_unary(tok_result.1)
            return (-inner.0, inner.1)
        }
        _ => return parse_primary(s)
    }
}

fn parse_primary(s: ParseState) -> (f64, ParseState) {
    let tok_result = next_token(s)
    match tok_result.0 {
        Token.Num(n) => return (n, tok_result.1)
        Token.LParen => {
            let inner = parse_expr(tok_result.1)
            // consume ')'
            let close = next_token(inner.1)
            return (inner.0, close.1)
        }
        _ => {
            panic("unexpected token in expression")
            return (0.0, tok_result.1)
        }
    }
}

fn eval(input: string) -> f64 {
    let s = ParseState { input: input, pos: 0 }
    let result = parse_expr(s)
    return result.0
}

fn main(stdout: Stdout) {
    stdout.println("=== Expression Calculator ===")

    let tests = [
        "2 + 3",
        "2 + 3 * 4",
        "(2 + 3) * 4",
        "10 / 2 - 3",
        "-5 + 3",
        "-(2 + 3)",
        "1.5 * 2",
        "100 / (5 * 2)",
        "3 + 4 * 2 / (1 - 5)",
        "2 * 3 + 4"
    ]

    let expected = [
        "5.0",
        "14.0",
        "20.0",
        "2.0",
        "-2.0",
        "-5.0",
        "3.0",
        "10.0",
        "1.0",
        "10.0"
    ]

    let mut passed = 0
    let mut failed = 0

    for i in 0..tests.length {
        let result = eval(tests[i])
        let result_str = result as string
        if result_str == expected[i] {
            passed = passed + 1
        } else {
            failed = failed + 1
            stdout.println("FAIL: " + tests[i] +
                " => " + result_str +
                " (expected " + expected[i] + ")")
        }
    }

    stdout.println(passed as string + " passed, " + failed as string + " failed")
}
