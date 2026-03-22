title: Types
date: 2026-03-02
description: Basalt's type system

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

See [Error Handling](error-handling.html) for the full story.

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
