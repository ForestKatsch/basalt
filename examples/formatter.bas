// Basalt source code formatter
// Reads Basalt source code and reformats it with consistent indentation.
//
// Rules:
// - 4 spaces per indent level
// - '{' increases indent for following lines
// - '}' decreases indent (applied to the '}' line itself)
// - Trim trailing whitespace
// - Collapse multiple blank lines into one
// - Don't count braces inside string literals

fn is_digit(c: string) -> bool {
    return c >= "0" && c <= "9"
}

fn is_alpha(c: string) -> bool {
    return (c >= "a" && c <= "z") ||
        (c >= "A" && c <= "Z") ||
        c == "_"
}

fn make_indent(level: i64) -> string {
    return "    ".repeat(level)
}

// Count the net brace depth change for a line, ignoring braces in strings
fn count_braces(line: string) -> (i64, i64) {
    let mut opens = 0
    let mut closes = 0
    let mut in_string = false
    let mut escaped = false

    for ch in line {
        if escaped {
            escaped = false
            continue
        }
        if ch == "\\" && in_string {
            escaped = true
            continue
        }
        if ch == "\"" {
            in_string = !in_string
            continue
        }
        if !in_string {
            if ch == "{" {
                opens = opens + 1
            }
            if ch == "}" {
                closes = closes + 1
            }
        }
    }

    return (opens, closes)
}

fn trim_line(line: string) -> string {
    return line.trim()
}

fn format_source(source: string) -> string {
    let lines = source.split("\n")
    let mut result = ""
    let mut indent = 0
    let mut prev_blank = false

    for line in lines {
        let trimmed = trim_line(line)

        // Handle blank lines - collapse multiples into one
        if trimmed.length == 0 {
            if !prev_blank {
                result = result + "\n"
                prev_blank = true
            }
            continue
        }
        prev_blank = false

        let braces = count_braces(trimmed)
        let opens = braces.0
        let closes = braces.1

        // Decrease indent for lines starting with '}'
        if trimmed.starts_with("}") {
            indent = indent - closes
            if indent < 0 { indent = 0 }
            result = result + make_indent(indent) + trimmed + "\n"
            indent = indent + opens
        } else {
            result = result + make_indent(indent) + trimmed + "\n"
            indent = indent + opens - closes
            if indent < 0 { indent = 0 }
        }
    }

    return result.trim()
}

fn main(stdout: Stdout) {
    // Test with some badly formatted Basalt code
    let ugly = "fn main(stdout: Stdout) {\nlet x = 42\nif x > 0 {\nstdout.println(\"positive\")\n} else {\nstdout.println(\"negative\")\n}\nfor i in 0..10 {\nlet doubled = i * 2\nstdout.println(doubled as string)\n}\n}"

    let formatted = format_source(ugly)
    stdout.println("=== Formatted Output ===")
    stdout.println(formatted)
    stdout.println("")

    // Test 2: nested types
    let ugly2 = "type Point {\nx: f64\ny: f64\nfn distance(self: Self, other: Point) -> f64 {\nlet dx = self.x - other.x\nlet dy = self.y - other.y\nreturn (dx * dx + dy * dy) ** 0.5\n}\n}"

    stdout.println("=== Formatted Type ===")
    stdout.println(format_source(ugly2))
    stdout.println("")

    // Test 3: already formatted code should be unchanged
    let clean =
        \\fn hello() {
        \\    stdout.println("world")
        \\}
    let reformatted = format_source(clean)
    if reformatted == clean {
        stdout.println("=== Idempotent: PASS ===")
    } else {
        stdout.println("=== Idempotent: FAIL ===")
        stdout.println("Expected:")
        stdout.println(clean)
        stdout.println("Got:")
        stdout.println(reformatted)
    }

    // Test 4: brace counting with strings
    let with_strings = "fn main() {\nlet s = \"hello { world }\"\nstdout.println(s)\n}"
    stdout.println("=== String Braces ===")
    stdout.println(format_source(with_strings))
}
