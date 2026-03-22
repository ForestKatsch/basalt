title: Standard Library
date: 2026-03-12
description: Built-in methods and the math module

### String Methods

| Method | Description |
|---|---|
| `s.length` | Character count (property, not a method) |
| `s.split(sep)` | Split by delimiter, returns `[string]` |
| `s.trim()` | Strip whitespace from both ends |
| `s.trim_start()` | Strip leading whitespace |
| `s.trim_end()` | Strip trailing whitespace |
| `s.replace(from, to)` | Replace all occurrences |
| `s.find(needle)` | Index of first match, returns `i64?` |
| `s.index_of(sub)` | Index of first occurrence |
| `s.last_index_of(sub)` | Index of last occurrence |
| `s.substring(start, len)` | Extract by char index and length |
| `s.slice(start, end)` | Extract by start/end index |
| `s.starts_with(prefix)` | Prefix test |
| `s.ends_with(suffix)` | Suffix test |
| `s.contains(sub)` | Substring test |
| `s.upper()` | Uppercase |
| `s.lower()` | Lowercase |
| `s.repeat(n)` | Repeat N times |
| `s.char_at(i)` | Single character by index (negative indexes from end) |
| `s.chars()` | Array of single-character strings |
| `s.bytes()` | Array of byte values |

```basalt
fn main(stdout: Stdout) {
    let s = "Hello, World!"
    stdout.println(s.upper())                  // Output: HELLO, WORLD!
    stdout.println(s.replace("World", "Basalt"))  // Output: Hello, Basalt!
    stdout.println(s.substring(0, 5))          // Output: Hello
    stdout.println(s.contains("World") as string)  // Output: true
    stdout.println("  spaces  ".trim())        // Output: spaces
    stdout.println("abc".repeat(3))            // Output: abcabcabc

    let parts = "a,b,c".split(",")
    stdout.println(parts.join(" | "))          // Output: a | b | c
}
```

String interpolation uses `\(expr)`:

```basalt
fn main(stdout: Stdout) {
    let name = "world"
    let n = 42
    stdout.println("Hello, \(name)!")         // Output: Hello, world!
    stdout.println("The answer is \(n)")      // Output: The answer is 42
    stdout.println("2 + 2 = \(2 + 2)")       // Output: 2 + 2 = 4
}
```

Multiline strings use `\\` prefix per line:

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

### Array Methods

| Method | Description |
|---|---|
| `arr.length` | Element count (property) |
| `arr.push(val)` | Append element (requires `mut`) |
| `arr.pop()` | Remove and return last element (requires `mut`) |
| `arr.insert(i, val)` | Insert at index (requires `mut`) |
| `arr.remove(i)` | Remove at index (requires `mut`) |
| `arr.sort()` | Sort in place (requires `mut`, `i64` or `string` only) |
| `arr.reverse()` | Reverse in place (requires `mut`) |
| `arr.join(sep)` | Join elements with separator |
| `arr.contains(val)` | Element membership test |
| `arr.clone()` | Deep copy |
| `arr.map(f)` | Transform each element |
| `arr.filter(f)` | Keep elements matching predicate |
| `arr.find(f)` | First element matching predicate |
| `arr.any(f)` | True if any element matches |
| `arr.all(f)` | True if all elements match |

```basalt
fn main(stdout: Stdout) {
    let mut nums = [3, 1, 4, 1, 5]

    nums.sort()
    stdout.println(nums.join(", "))  // Output: 1, 1, 3, 4, 5

    nums.reverse()
    stdout.println(nums.join(", "))  // Output: 5, 4, 3, 1, 1

    stdout.println(nums.contains(3) as string)  // Output: true

    // Negative indexing
    stdout.println(nums[-1] as string)  // Output: 1 (last element)
}
```

### Map Methods

| Method | Description |
|---|---|
| `m.length` | Entry count (property) |
| `m[key]` | Get value (panics if key missing) |
| `m[key] = val` | Set value (requires `mut`) |
| `m.get(key)` | Safe lookup, returns `V?` (nil if missing) |
| `m.contains_key(key)` | Key membership test |
| `m.keys()` | Array of keys (insertion order) |
| `m.values()` | Array of values (insertion order) |
| `m.remove(key)` | Remove entry (requires `mut`) |
| `m.clone()` | Deep copy |

```basalt
fn main(stdout: Stdout) {
    let mut m = {"name": "Alice", "city": "NYC"}

    stdout.println(m["name"])  // Output: Alice

    // Safe lookup (does not panic)
    let age = m.get("age")
    if age is nil {
        stdout.println("no age set")  // Output: no age set
    }

    m["age"] = "30"
    stdout.println(m.keys().join(", "))  // Output: name, city, age

    // Iterate
    for key, value in m {
        stdout.println("\(key): \(value)")
    }
}
```

Empty maps need a type annotation:

```basalt
let mut scores: [string: i64] = {}
scores["alice"] = 100
```

### Math Module

```basalt
import "std/math"
```

| Function | Description |
|---|---|
| `math.sqrt(x)` | Square root |
| `math.abs(x)` | Absolute value |
| `math.floor(x)` | Floor |
| `math.ceil(x)` | Ceiling |
| `math.round(x)` | Round |
| `math.min(a, b)` | Minimum |
| `math.max(a, b)` | Maximum |
| `math.pow(base, exp)` | Power |
| `math.log(x)` | Natural log |
| `math.log2(x)` | Base-2 log |
| `math.log10(x)` | Base-10 log |
| `math.exp(x)` | e^x |
| `math.sin(x)`, `math.cos(x)`, `math.tan(x)` | Trigonometric |
| `math.asin(x)`, `math.acos(x)`, `math.atan(x)` | Inverse trig |
| `math.atan2(y, x)` | Two-argument arctangent |
| `math.pi`, `math.e`, `math.tau`, `math.inf` | Constants |
