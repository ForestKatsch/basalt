title: Standard Library
date: 2026-03-12
description: Method reference for strings, arrays, maps, and the math module.

Basalt's standard library is intentionally small. String, array, and map methods are built in — no imports needed. The `math` module is the only thing you import.

## Strings

Strings are immutable, UTF-8 encoded sequences. All methods return new values.

| Method | Description |
|---|---|
| `s.length` | Character count (property) |
| `s.split(sep)` | Split into array by delimiter |
| `s.trim()` | Strip whitespace from both ends |
| `s.trim_start()` | Strip leading whitespace |
| `s.trim_end()` | Strip trailing whitespace |
| `s.replace(from, to)` | Replace all occurrences |
| `s.contains(sub)` | Substring test |
| `s.starts_with(prefix)` | Prefix test |
| `s.ends_with(suffix)` | Suffix test |
| `s.upper()` | Uppercase |
| `s.lower()` | Lowercase |
| `s.repeat(n)` | Repeat N times |
| `s.substring(start, len)` | Extract by start index and length |
| `s.slice(start, end)` | Extract by start and end index |
| `s.index_of(sub)` | Index of first occurrence |
| `s.last_index_of(sub)` | Index of last occurrence |
| `s.char_at(i)` | Character at index (negative indexes from end) |
| `s.chars()` | Array of single-character strings |
| `s.bytes()` | Array of byte values |

```basalt
fn main(stdout: Stdout) {
    let path = "/users/alice/documents/report.txt"
    let parts = path.split("/")
    let filename = parts[-1]
    stdout.println(filename)                    // Output: report.txt
    stdout.println(filename.upper())            // Output: REPORT.TXT
    stdout.println(path.contains("alice") as string)  // Output: true

    let csv = "  name, age, city  "
    stdout.println(csv.trim())                  // Output: name, age, city
    stdout.println("ha".repeat(3))              // Output: hahaha
}
```

### String interpolation

Use `\(expr)` inside double-quoted strings to embed any expression:

```basalt
fn main(stdout: Stdout) {
    let name = "Basalt"
    let version = 1
    stdout.println("Welcome to \(name) v\(version)")  // Output: Welcome to Basalt v1
    stdout.println("2 + 2 = \(2 + 2)")                // Output: 2 + 2 = 4
}
```

### Multiline strings

Prefix each line with `\\` for multiline string literals:

```basalt
fn main(stdout: Stdout) {
    let json =
        \\{
        \\    "name": "basalt",
        \\    "version": 1
        \\}
    stdout.println(json)
}
```

## Arrays

Arrays are ordered, homogeneous collections. They grow dynamically.

| Method | Description |
|---|---|
| `arr.length` | Element count (property) |
| `arr.push(val)` | Append element (requires `mut`) |
| `arr.pop()` | Remove and return last (requires `mut`) |
| `arr.insert(i, val)` | Insert at index (requires `mut`) |
| `arr.remove(i)` | Remove at index (requires `mut`) |
| `arr.sort()` | Sort in place (requires `mut`) |
| `arr.reverse()` | Reverse in place (requires `mut`) |
| `arr.join(sep)` | Join elements with separator |
| `arr.contains(val)` | Element membership test |
| `arr.clone()` | Deep copy |
| `arr.map(f)` | Transform each element |
| `arr.filter(f)` | Keep matching elements |
| `arr.find(f)` | First matching element (returns optional) |
| `arr.any(f)` | True if any element matches |
| `arr.all(f)` | True if all match |

```basalt
fn main(stdout: Stdout) {
    let mut tags = ["draft", "review", "urgent"]
    tags.push("published")
    tags.sort()
    stdout.println(tags.join(", "))  // Output: draft, published, review, urgent

    stdout.println(tags.contains("review") as string)  // Output: true
    stdout.println(tags[-1])  // Output: urgent (negative indexing)

    let upper = tags.map(fn(t: string) -> string { return t.upper() })
    stdout.println(upper[0])  // Output: DRAFT
}
```

## Maps

Maps are ordered key-value collections. Keys are strings or integers.

| Method | Description |
|---|---|
| `m.length` | Entry count (property) |
| `m[key]` | Get value (panics if missing) |
| `m[key] = val` | Set value (requires `mut`) |
| `m.get(key)` | Safe lookup, returns optional |
| `m.contains_key(key)` | Key membership test |
| `m.keys()` | Array of keys (insertion order) |
| `m.values()` | Array of values (insertion order) |
| `m.remove(key)` | Remove entry (requires `mut`) |
| `m.clone()` | Deep copy |

```basalt
fn main(stdout: Stdout) {
    let mut headers = {"content-type": "text/html", "status": "200"}
    headers["cache-control"] = "no-cache"

    for key, value in headers {
        stdout.println("\(key): \(value)")
    }
    // Output: content-type: text/html
    // Output: status: 200
    // Output: cache-control: no-cache

    let ct = headers.get("content-type")
    if ct is string {
        stdout.println("Found: " + (ct as string))  // Output: Found: text/html
    }
}
```

Empty maps need a type annotation:

```basalt
let mut scores: [string: i64] = {}
scores["alice"] = 100
```

## Math module

```basalt
import "std/math"
```

| Function | Description |
|---|---|
| `math.sqrt(x)` | Square root |
| `math.abs(x)` | Absolute value |
| `math.floor(x)` | Floor |
| `math.ceil(x)` | Ceiling |
| `math.round(x)` | Round to nearest |
| `math.min(a, b)` | Minimum |
| `math.max(a, b)` | Maximum |
| `math.pow(base, exp)` | Power |
| `math.log(x)`, `math.log2(x)`, `math.log10(x)` | Logarithms |
| `math.sin(x)`, `math.cos(x)`, `math.tan(x)` | Trigonometric |
| `math.pi`, `math.e`, `math.tau`, `math.inf` | Constants |

```basalt
import "std/math"

fn main(stdout: Stdout) {
    let hypotenuse = math.sqrt(3.0 * 3.0 + 4.0 * 4.0)
    stdout.println(hypotenuse as string)  // Output: 5

    let clamped = math.max(0.0, math.min(100.0, 150.0))
    stdout.println(clamped as string)  // Output: 100
}
```

## What's Next

You now know the tools the language gives you. [Capabilities](capabilities.html) explain how Basalt controls what your program is allowed to do in the outside world.
