// JSON parser and pretty-printer written in Basalt
// Parses a JSON string into a tree of Json values, then pretty-prints it.

type Json {
    Null
    Bool(bool)
    Num(f64)
    Str(string)
    Arr([Json])
    Obj([string: Json])
}

type Parser {
    input: string
    pos: i64
}

fn new_parser(input: string) -> Parser {
    return Parser { input: input, pos: 0 }
}

fn peek(p: Parser) -> string {
    if p.pos >= p.input.length { return "" }
    return p.input.char_at(p.pos)
}

fn adv(p: Parser) -> Parser {
    return Parser { input: p.input, pos: p.pos + 1 }
}

fn skip_ws(p: Parser) -> Parser {
    let mut s = p
    loop {
        let c = peek(s)
        if c == " " || c == "\n" || c == "\t" || c == "\r" {
            s = adv(s)
        } else {
            break
        }
    }
    return s
}

fn expect(p: Parser, ch: string) -> Parser!string {
    let s = skip_ws(p)
    if peek(s) != ch {
        return !("expected '" + ch + "' at position " + (s.pos as string))
    }
    return adv(s)
}

fn parse_json(p: Parser) -> (Json, Parser)!string {
    let s = skip_ws(p)
    let c = peek(s)

    if c == "n" { return parse_null(s) }
    if c == "t" || c == "f" { return parse_bool(s) }
    if c == "\"" { return parse_string(s) }
    if c == "[" { return parse_array(s) }
    if c == "{" { return parse_object(s) }
    if c == "-" || (c >= "0" && c <= "9") { return parse_number(s) }

    return !("unexpected character '" + c + "' at position " + (s.pos as string))
}

fn parse_null(p: Parser) -> (Json, Parser)!string {
    // consume "null"
    let mut s = p
    let word = "null"
    for i in 0..4 {
        if peek(s) != word.char_at(i) {
            return !("expected 'null'")
        }
        s = adv(s)
    }
    return (Json.Null, s)
}

fn parse_bool(p: Parser) -> (Json, Parser)!string {
    let mut s = p
    if peek(s) == "t" {
        for i in 0..4 {
            if peek(s) != "true".char_at(i) {
                return !("expected 'true'")
            }
            s = adv(s)
        }
        return (Json.Bool(true), s)
    }
    for i in 0..5 {
        if peek(s) != "false".char_at(i) {
            return !("expected 'false'")
        }
        s = adv(s)
    }
    return (Json.Bool(false), s)
}

fn parse_number(p: Parser) -> (Json, Parser)!string {
    let mut s = p
    let mut num_str = ""

    if peek(s) == "-" {
        num_str = "-"
        s = adv(s)
    }

    while peek(s) >= "0" && peek(s) <= "9" {
        num_str = num_str + peek(s)
        s = adv(s)
    }

    if peek(s) == "." {
        num_str = num_str + "."
        s = adv(s)
        while peek(s) >= "0" && peek(s) <= "9" {
            num_str = num_str + peek(s)
            s = adv(s)
        }
    }

    if num_str.length == 0 || num_str == "-" {
        return !("invalid number")
    }

    return (Json.Num(num_str as f64), s)
}

fn parse_string(p: Parser) -> (Json, Parser)!string {
    let mut s = adv(p) // skip opening quote
    let mut result = ""

    loop {
        let c = peek(s)
        if c == "" { return !("unterminated string") }
        if c == "\"" {
            return (Json.Str(result), adv(s))
        }
        if c == "\\" {
            s = adv(s)
            let esc = peek(s)
            if esc == "\"" { result = result + "\"" }
            else if esc == "\\" { result = result + "\\" }
            else if esc == "n" { result = result + "\n" }
            else if esc == "t" { result = result + "\t" }
            else { result = result + esc }
            s = adv(s)
        } else {
            result = result + c
            s = adv(s)
        }
    }
}

fn parse_array(p: Parser) -> (Json, Parser)!string {
    let mut s = adv(p) // skip '['
    s = skip_ws(s)
    let mut items: [Json] = []

    if peek(s) == "]" {
        return (Json.Arr(items), adv(s))
    }

    loop {
        let result = parse_json(s)?
        items.push(result.0)
        s = skip_ws(result.1)

        if peek(s) == "," {
            s = adv(s)
        } else {
            break
        }
    }

    s = expect(s, "]")?
    return (Json.Arr(items), s)
}

fn parse_object(p: Parser) -> (Json, Parser)!string {
    let mut s = adv(p) // skip '{'
    s = skip_ws(s)
    let mut entries: [string: Json] = {}

    if peek(s) == "}" {
        return (Json.Obj(entries), adv(s))
    }

    loop {
        // Parse key (must be string)
        s = skip_ws(s)
        if peek(s) != "\"" {
            return !("expected string key at " + (s.pos as string))
        }
        let key_result = parse_string(s)?
        let key = match key_result.0 {
            Json.Str(k) => k
            _ => return !("object key must be string")
        }
        s = key_result.1

        s = expect(s, ":")?
        let val_result = parse_json(s)?
        entries[key] = val_result.0
        s = skip_ws(val_result.1)

        if peek(s) == "," {
            s = adv(s)
        } else {
            break
        }
    }

    s = expect(s, "}")?
    return (Json.Obj(entries), s)
}

// Pretty printer
fn pretty(j: Json, indent: i64) -> string {
    let pad = "  ".repeat(indent)
    let pad1 = "  ".repeat(indent + 1)

    match j {
        Json.Null => return "null"
        Json.Bool(b) => return b as string
        Json.Num(n) => return n as string
        Json.Str(s) => return "\"" + s + "\""
        Json.Arr(items) => {
            if items.length == 0 { return "[]" }
            let mut parts = "[\n"
            for i in 0..items.length {
                parts = parts + pad1 + pretty(items[i], indent + 1)
                if i < items.length - 1 {
                    parts = parts + ","
                }
                parts = parts + "\n"
            }
            return parts + pad + "]"
        }
        Json.Obj(entries) => {
            if entries.length == 0 { return "{}" }
            let keys = entries.keys()
            let mut parts = "{\n"
            for i in 0..keys.length {
                let k = keys[i]
                let v = entries[k]
                parts = parts + pad1 + "\"" + k + "\": " + pretty(v, indent + 1)
                if i < keys.length - 1 {
                    parts = parts + ","
                }
                parts = parts + "\n"
            }
            return parts + pad + "}"
        }
    }
}

fn main(stdout: Stdout) {
    let input = "{\"name\": \"Basalt\", \"version\": 1.0, \"features\": [\"types\", \"closures\", true], \"nested\": {\"a\": 1, \"b\": null}}"

    stdout.println("=== Input ===")
    stdout.println(input)
    stdout.println("")

    match parse_json(new_parser(input)) {
        !err => stdout.println("Parse error: " + err)
        result => {
            stdout.println("=== Pretty Printed ===")
            stdout.println(pretty(result.0, 0))
        }
    }

    // Test edge cases
    stdout.println("")
    stdout.println("=== Edge Cases ===")

    let cases = [
        "null",
        "true",
        "42",
        "\"hello\"",
        "[]",
        "{}",
        "[1, 2, 3]",
        "{\"key\": \"value\"}"
    ]

    for c in cases {
        match parse_json(new_parser(c)) {
            !err => stdout.println("ERROR: " + err)
            result => stdout.println(c + " => " + pretty(result.0, 0))
        }
    }
}
