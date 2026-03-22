# The Basalt Language Guide

Basalt is a statically typed, capability-based programming language. This guide teaches you how to write Basalt programs through practical examples. For exhaustive detail, see the [Language Specification](SPEC.md).

## Getting Started

Build the compiler from source:

```sh
cargo build --release
```

Run a program:

```sh
basalt run hello.bas
```

Type-check without running:

```sh
basalt check hello.bas
```

Your first program:

```basalt
fn main(stdout: Stdout) {
    stdout.println("Hello, Basalt!")
}
```

Every program needs a `main` function. IO is performed through capabilities passed as parameters — here, `Stdout` lets us print. If you don't request a capability, you can't use it.

## Types

### Primitive Types

```basalt
fn main(stdout: Stdout) {
    let i = 42              // i64 (default integer)
    let f = 3.14            // f64
    let b = true            // bool
    let s = "hello"         // string (immutable, UTF-8)
    let n = nil             // nil (absence of value)

    // Sized integer types require annotation
    let byte: u8 = 255
    let small: i16 = -1000
    let id: u64 = 0xDEADBEEF

    stdout.println(i as string)     // Output: 42
    stdout.println(byte as string)  // Output: 255
}
```

Integer types: `i8`, `i16`, `i32`, `i64` (signed) and `u8`, `u16`, `u32`, `u64` (unsigned). All integer literals default to `i64`. Integer arithmetic is checked — overflow panics at runtime.

There is one float type: `f64` (IEEE 754 double precision).

### Collections

```basalt
fn main(stdout: Stdout) {
    // Arrays: ordered, growable, homogeneous
    let nums = [1, 2, 3]
    stdout.println(nums[0] as string)  // Output: 1

    // Maps: ordered by insertion, key-value pairs
    let ages = {"alice": 30, "bob": 25}
    stdout.println(ages["alice"] as string)  // Output: 30

    // Tuples: fixed-size, heterogeneous, immutable
    let pair = (42, "hello")
    stdout.println(pair.0 as string)  // Output: 42
    stdout.println(pair.1)            // Output: hello
}
```

Arrays and maps are reference types — assignment shares the same object. Use `.clone()` for an independent copy. Tuples are value types — assignment copies.

```basalt
fn main(stdout: Stdout) {
    let mut a = [1, 2, 3]
    let mut b = a         // b and a point to the same array
    b.push(4)
    stdout.println(a.length as string)  // Output: 4 (same object!)

    let mut c = a.clone() // independent copy
    c.push(5)
    stdout.println(a.length as string)  // Output: 4 (unaffected)
}
```

### Optional Types

`T?` represents a value that might be absent. It is equivalent to `T | nil`.

```basalt
fn find_user(id: i64) -> string? {
    if id == 1 { return "Alice" }
    return nil
}

fn main(stdout: Stdout) {
    let name = find_user(1)
    if name is string {
        stdout.println("Found: " + name)  // Output: Found: Alice
    }

    let missing = find_user(99)
    if missing is nil {
        stdout.println("Not found")  // Output: Not found
    }
}
```

### Result Types

`T!E` represents either a success value of type `T` or an error of type `E`. Errors are values, not exceptions.

```basalt
fn parse_port(s: string) -> i64!string {
    if s.length == 0 { return !("empty input") }
    let n = s as? i64
    if n is nil { return !("not a number: " + s) }
    return n as i64
}

fn main(stdout: Stdout) {
    match parse_port("8080") {
        !err => stdout.println("Error: " + err)
        port => stdout.println("Port: " + (port as string))
    }
    // Output: Port: 8080

    match parse_port("abc") {
        !err => stdout.println("Error: " + err)
        _ => stdout.println("OK")
    }
    // Output: Error: not a number: abc
}
```

See [Error Handling](#error-handling) for the full story.

### Union Types

A union `A | B` accepts values of either type. Use `is` to narrow to a specific member.

```basalt
fn describe(val: i64 | string) -> string {
    if val is i64 {
        return "number: " + (val as string)
    } else {
        return "text: " + val
    }
}

fn main(stdout: Stdout) {
    stdout.println(describe(42))       // Output: number: 42
    stdout.println(describe("hello"))  // Output: text: hello
}
```

You can name unions with type aliases:

```basalt
type Numeric = i64 | f64
type JsonPrimitive = bool | f64 | string | nil
```

## Variables

Variables are immutable by default:

```basalt
fn main(stdout: Stdout) {
    let name = "Basalt"
    let age = 1
    stdout.println("\(name) is \(age) year old")
    // Output: Basalt is 1 year old
}
```

Use `mut` when you need to reassign or mutate:

```basalt
fn main(stdout: Stdout) {
    let mut count = 0
    count = count + 1
    stdout.println(count as string)  // Output: 1

    // mut is also required for mutating methods on collections
    let mut items = [1, 2, 3]
    items.push(4)
    stdout.println(items.length as string)  // Output: 4
}
```

Type inference works for most declarations. Add annotations when needed:

```basalt
fn main(stdout: Stdout) {
    let x = 42                  // inferred as i64
    let y: f64 = 42.0           // annotated
    let byte: u8 = 255          // required for non-default integer widths
    let mut m: [string: i64] = {}  // required for empty collections

    stdout.println(x as string)
}
```

## Functions

```basalt
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn greet(name: string, stdout: Stdout) {
    stdout.println("Hello, \(name)!")
}

fn main(stdout: Stdout) {
    let sum = add(3, 4)
    stdout.println(sum as string)  // Output: 7
    greet("world", stdout)         // Output: Hello, world!
}
```

All parameters need type annotations. Return type is required unless the function returns `nil`. Explicit `return` is required — the last expression is not implicitly returned.

Functions are first-class values:

```basalt
fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
    return f(x)
}

fn double(x: i64) -> i64 {
    return x * 2
}

fn main(stdout: Stdout) {
    let result = apply(double, 5)
    stdout.println(result as string)  // Output: 10
}
```

## Control Flow

### if/else

Conditions must be `bool`. There is no truthiness — `if 0` and `if ""` are compile errors.

`if` is an expression that produces a value:

```basalt
fn main(stdout: Stdout) {
    let x = 10
    let label = if x > 0 { "positive" } else { "non-positive" }
    stdout.println(label)  // Output: positive

    let grade = if x >= 90 { "A" }
        else if x >= 80 { "B" }
        else if x >= 70 { "C" }
        else { "F" }
    stdout.println(grade)  // Output: F
}
```

### while

```basalt
fn main(stdout: Stdout) {
    let mut i = 1
    while i <= 5 {
        stdout.print(i as string + " ")
        i = i + 1
    }
    stdout.println("")
    // Output: 1 2 3 4 5
}
```

### for-in

Iterate over arrays, maps, strings, and ranges:

```basalt
fn main(stdout: Stdout) {
    // Array iteration
    let fruits = ["apple", "banana", "cherry"]
    for fruit in fruits {
        stdout.println(fruit)
    }

    // With index (value first, index second)
    for fruit, i in fruits {
        stdout.println("\(i as string): \(fruit)")
    }

    // Map iteration (key first, value second)
    let ages = {"alice": 30, "bob": 25}
    for name, age in ages {
        stdout.println("\(name) is \(age as string)")
    }

    // Range (exclusive end)
    let mut sum = 0
    for i in 0..10 {
        sum = sum + i
    }
    stdout.println(sum as string)  // Output: 45

    // String iteration (by character)
    for ch in "hi!" {
        stdout.print("[" + ch + "]")
    }
    stdout.println("")  // Output: [h][i][!]
}
```

### loop, break, continue

```basalt
fn main(stdout: Stdout) {
    // Infinite loop with break
    let mut n = 0
    loop {
        n = n + 1
        if n > 5 { break }
    }
    stdout.println(n as string)  // Output: 6

    // Skip odd numbers with continue
    let mut evens = 0
    for i in 0..10 {
        if i % 2 != 0 { continue }
        evens = evens + 1
    }
    stdout.println(evens as string)  // Output: 5
}
```

### guard / guard let

`guard` asserts a condition. If it fails, the `else` block must diverge (`return`, `break`, `continue`, or `panic`).

```basalt
fn process(x: i64, stdout: Stdout) {
    guard x > 0 else {
        stdout.println("must be positive")
        return
    }
    stdout.println("processing: " + (x as string))
}

fn main(stdout: Stdout) {
    process(5, stdout)   // Output: processing: 5
    process(-1, stdout)  // Output: must be positive
}
```

`guard let` unwraps optionals and results into the enclosing scope:

```basalt
fn load_config(fs: Fs, stdout: Stdout) {
    guard let content = fs.read_file("config.txt") else {
        stdout.println("Could not read config")
        return
    }
    // content is `string` here, not a result type
    stdout.println("Config loaded: " + (content.length as string) + " chars")
}
```

## Pattern Matching

`match` is an expression with exhaustive checking. It works on literals, enums, results, and union types.

### Literal patterns

```basalt
fn classify(n: i64) -> string {
    match n {
        0 => return "zero"
        1 => return "one"
        _ => return "other"
    }
    return "unreachable"
}

fn main(stdout: Stdout) {
    stdout.println(classify(0))   // Output: zero
    stdout.println(classify(42))  // Output: other
}
```

### Enum variant patterns

```basalt
type Shape {
    Circle(f64)
    Rect(f64, f64)
}

fn area(s: Shape) -> f64 {
    match s {
        Shape.Circle(r) => return 3.14159 * r * r
        Shape.Rect(w, h) => return w * h
    }
    return 0.0
}

fn main(stdout: Stdout) {
    let c = Shape.Circle(5.0)
    stdout.println(area(c) as string)  // Output: 78.53975
}
```

### Type narrowing with is

Use `is` patterns to match union members:

```basalt
fn to_string(val: i64 | f64 | string) -> string {
    match val {
        is i64 => return "int: " + (val as string)
        is f64 => return "float: " + (val as string)
        is string => return "str: " + val
    }
    return ""
}
```

### Matching results

```basalt
fn divide(a: f64, b: f64) -> f64!string {
    if b == 0.0 { return !("division by zero") }
    return a / b
}

fn main(stdout: Stdout) {
    match divide(10.0, 3.0) {
        !err => stdout.println("Error: " + err)
        val => stdout.println("Result: " + (val as string))
    }
    // Output: Result: 3.3333333333333335
}
```

## Error Handling

Basalt has no exceptions. Errors are values, carried by result types `T!E`.

### Creating and returning errors

```basalt
fn parse_age(s: string) -> i64!string {
    let n = s as? i64
    if n is nil {
        return !("invalid number: " + s)
    }
    let age = n as i64
    if age < 0 {
        return !("age cannot be negative")
    }
    return age
}
```

### Propagating with ?

The `?` operator propagates errors automatically. If the result is an error, it returns from the enclosing function. If successful, it unwraps the value.

```basalt
fn load_and_parse(fs: Fs) -> i64!string {
    let content = fs.read_file("age.txt")?  // propagates read error
    let age = parse_age(content.trim())?     // propagates parse error
    return age
}
```

The enclosing function must return a compatible result type.

### guard let for unwrapping

When you want to unwrap and diverge on failure without `match`:

```basalt
fn process(fs: Fs, stdout: Stdout) {
    guard let data = fs.read_file("input.txt") else {
        stdout.println("Cannot read input")
        return
    }
    // data is `string` here
    stdout.println("Read " + (data.length as string) + " chars")
}
```

### panic for unrecoverable errors

```basalt
fn main(stdout: Stdout) {
    panic("something went terribly wrong")
    // Terminates with message and stack trace. Cannot be caught.
}
```

Use `panic` for programming errors. Use result types for expected failures.

## Structs

Define structs with the `type` keyword. Structs are reference types.

```basalt
type Point {
    x: f64
    y: f64
}

fn main(stdout: Stdout) {
    let p = Point { x: 3.0, y: 4.0 }
    stdout.println(p.x as string)  // Output: 3
    stdout.println(p.y as string)  // Output: 4
}
```

### Methods

Instance methods take `self: Self` as their first parameter. Static methods do not.

```basalt
type Point {
    x: f64
    y: f64

    fn origin() -> Point {
        return Point { x: 0.0, y: 0.0 }
    }

    fn distance(self: Self, other: Point) -> f64 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        return math.sqrt(dx * dx + dy * dy)
    }
}

fn main(stdout: Stdout) {
    let a = Point.origin()
    let b = Point { x: 3.0, y: 4.0 }
    stdout.println(a.distance(b) as string)  // Output: 5
}
```

### Reference semantics

```basalt
fn main(stdout: Stdout) {
    let mut p = Point { x: 1.0, y: 2.0 }
    let mut q = p        // q and p refer to the same struct
    q.x = 10.0
    stdout.println(p.x as string)  // Output: 10 (same object)

    let mut r = p.clone()  // independent copy
    r.x = 99.0
    stdout.println(p.x as string)  // Output: 10 (unaffected)
}
```

## Enums

Enums define a closed set of variants. Variants can carry data.

```basalt
type Color { Red, Green, Blue }

type Option {
    Some(i64)
    None
}

fn main(stdout: Stdout) {
    let c = Color.Red

    let x = Option.Some(42)
    match x {
        Option.Some(val) => stdout.println(val as string)
        Option.None => stdout.println("none")
    }
    // Output: 42
}
```

### Data-carrying variants

```basalt
type Shape {
    Circle(f64)
    Rect(f64, f64)
}

fn describe(s: Shape) -> string {
    match s {
        Shape.Circle(r) => return "circle with radius " + (r as string)
        Shape.Rect(w, h) => return (w as string) + "x" + (h as string) + " rectangle"
    }
    return ""
}

fn main(stdout: Stdout) {
    stdout.println(describe(Shape.Circle(5.0)))
    // Output: circle with radius 5
    stdout.println(describe(Shape.Rect(3.0, 4.0)))
    // Output: 3x4 rectangle
}
```

### Testing variants with is

```basalt
fn main(stdout: Stdout) {
    let x = Option.Some(42)
    if x is Option.Some {
        stdout.println("has a value")
    }
    // Use match to destructure the inner data
}
```

### Recursive enums

Enum variants can reference the enclosing type, enabling trees and nested data:

```basalt
type Json {
    Null
    Bool(bool)
    Num(f64)
    Str(string)
    Arr([Json])
    Obj([string: Json])
}
```

## Closures

Closures use the `fn` keyword with inline bodies. They capture variables from their enclosing scope by reference.

```basalt
fn main(stdout: Stdout) {
    let double = fn(x: i64) -> i64 { return x * 2 }
    let square = fn(x: i64) -> i64 { return x * x }

    stdout.println(double(5) as string)  // Output: 10
    stdout.println(square(5) as string)  // Output: 25
}
```

### Capture by reference

Mutations through closures are visible in the enclosing scope:

```basalt
fn main(stdout: Stdout) {
    let mut count = 0
    let increment = fn() {
        count = count + 1
    }
    increment()
    increment()
    stdout.println(count as string)  // Output: 2
}
```

### Functional array methods

Arrays support `map`, `filter`, `find`, `any`, and `all`:

```basalt
fn main(stdout: Stdout) {
    let nums = [1, 2, 3, 4, 5]

    // map: transform each element
    let doubled = nums.map(fn(x: i64) -> i64 { return x * 2 })
    stdout.println(doubled.join(", "))  // Output: 2, 4, 6, 8, 10

    // filter: keep matching elements
    let evens = nums.filter(fn(x: i64) -> bool { return x % 2 == 0 })
    stdout.println(evens.join(", "))  // Output: 2, 4

    // any / all
    let has_big = nums.any(fn(x: i64) -> bool { return x > 3 })
    stdout.println(has_big as string)  // Output: true

    let all_pos = nums.all(fn(x: i64) -> bool { return x > 0 })
    stdout.println(all_pos as string)  // Output: true
}
```

## Modules

Each `.bas` file is a module. Import with the `import` keyword.

```basalt
// geometry.bas
type Point {
    x: f64
    y: f64
}

fn distance(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x
    let dy = a.y - b.y
    return math.sqrt(dx * dx + dy * dy)
}
```

```basalt
// main.bas
import "geometry"

fn main(stdout: Stdout) {
    let a = geometry.Point { x: 0.0, y: 0.0 }
    let b = geometry.Point { x: 3.0, y: 4.0 }
    stdout.println(geometry.distance(a, b) as string)
}
```

All imported names are accessed with the module qualifier. There is no unqualified import. Circular imports are prohibited.

Import with alias:

```basalt
import "lib/utils" as helpers
```

Standard library modules use the `std/` prefix:

```basalt
import "std/math"

fn main(stdout: Stdout) {
    stdout.println(math.sqrt(2.0) as string)
}
```

## Standard Library

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

## Capabilities

IO in Basalt works through capability objects passed to `main`. A program can only perform IO that it explicitly requests.

### Stdout

```basalt
fn main(stdout: Stdout) {
    stdout.println("line with newline")
    stdout.print("no newline")
    stdout.flush()
}
```

### Stdin

```basalt
fn main(stdout: Stdout, stdin: Stdin) {
    stdout.print("What is your name? ")
    stdout.flush()
    let name = stdin.read_line()
    stdout.println("Hello, \(name)!")
}
```

| Method | Description |
|---|---|
| `stdin.read_line()` | Read one line (strips newline) |
| `stdin.read_key()` | Read single character (raw mode) |

### Fs (File System)

The `Fs` capability is sandboxed to the directory containing the source file.

```basalt
fn main(stdout: Stdout, fs: Fs) {
    // Write a file
    let write_result = fs.write_file("output.txt", "Hello from Basalt!")
    match write_result {
        !err => stdout.println("Write failed: " + err)
        _ => stdout.println("Wrote output.txt")
    }

    // Read a file
    guard let content = fs.read_file("output.txt") else {
        stdout.println("Read failed")
        return
    }
    stdout.println(content)

    // Check existence
    stdout.println(fs.exists("output.txt") as string)  // Output: true

    // List directory
    guard let files = fs.read_dir(".") else { return }
    for file in files {
        stdout.println(file)
    }

    // Path utilities
    let p = fs.join("subdir", "page.html")   // "subdir/page.html"
    let ext = fs.extension("photo.png")       // "png" (returns string?)
    let name = fs.stem("photo.png")           // "photo" (returns string?)
}
```

| Method | Returns | Description |
|---|---|---|
| `fs.read_file(path)` | `string!string` | Read file contents |
| `fs.write_file(path, data)` | `nil!string` | Write file |
| `fs.read_dir(path)` | `[string]!string` | List directory entries |
| `fs.exists(path)` | `bool` | Check if path exists |
| `fs.is_dir(path)` | `bool` | Check if path is a directory |
| `fs.mkdir(path)` | `nil!string` | Create directory |
| `fs.join(a, b)` | `string` | Join path components |
| `fs.extension(path)` | `string?` | File extension without dot |
| `fs.stem(path)` | `string?` | Filename without extension |

### Env

```basalt
fn main(stdout: Stdout, env: Env) {
    let args = env.args()
    for arg in args {
        stdout.println(arg)
    }

    let home = env.get("HOME")
    if home is string {
        stdout.println("Home: " + home)
    }
}
```

| Method | Returns | Description |
|---|---|---|
| `env.args()` | `[string]` | Command-line arguments |
| `env.get(name)` | `string?` | Environment variable value |

### How capabilities work

Capabilities are passed as parameters to `main`. You only get what you ask for:

```basalt
// This program can print but cannot read files
fn main(stdout: Stdout) {
    stdout.println("I have no file access")
}

// This program can read/write files and print
fn main(stdout: Stdout, fs: Fs) {
    guard let data = fs.read_file("input.txt") else { return }
    stdout.println(data)
}
```

Pass capabilities to helper functions as regular parameters:

```basalt
fn log(msg: string, stdout: Stdout) {
    stdout.println("[LOG] " + msg)
}

fn main(stdout: Stdout) {
    log("starting up", stdout)
}
```

## Type Conversions

### as (strict)

Converts the value or panics if impossible:

```basalt
fn main(stdout: Stdout) {
    stdout.println(42 as string)        // Output: 42
    stdout.println(3.14 as string)      // Output: 3.14
    stdout.println(true as string)      // Output: true

    let x = 42 as f64                   // 42.0
    let n = 3.9 as i64                  // 3 (truncates toward zero)
    let parsed = "42" as i64            // 42

    stdout.println(n as string)         // Output: 3
    // "hello" as i64                   // PANIC: cannot parse
    // 300 as u8                        // PANIC: out of range
}
```

### as? (safe)

Returns `T?` — the converted value or `nil` on failure:

```basalt
fn main(stdout: Stdout) {
    let good = "42" as? i64       // 42
    let bad = "hello" as? i64     // nil
    let fits = 255 as? u8         // 255
    let overflow = 300 as? u8     // nil

    if good is i64 {
        stdout.println("parsed: " + (good as string))
    }
    if bad is nil {
        stdout.println("parse failed")
    }
    // Output:
    // parsed: 42
    // parse failed
}
```

### Conversion table

| From | To | Behavior |
|---|---|---|
| any integer | any integer | Range-checked (panics/nil on overflow) |
| any integer | `f64` | Always succeeds (widening) |
| `f64` | any integer | Truncates toward zero, range-checked |
| any numeric | `string` | Display representation |
| `string` | any numeric | Parses (panics/nil on invalid input) |
| `bool` | `string` | `"true"` or `"false"` |

Cross-width arithmetic is a type error. Convert explicitly:

```basalt
fn main(stdout: Stdout) {
    let a: i32 = 10
    let b: i64 = 20
    // let c = a + b        // COMPILE ERROR: type mismatch
    let c = (a as i64) + b  // correct
    stdout.println(c as string)  // Output: 30
}
```
