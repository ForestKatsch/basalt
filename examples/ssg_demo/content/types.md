title: Types
date: 2026-03-02
description: Integers, floats, strings, arrays, maps, optionals, results, and unions.

Every value in Basalt has a type known at compile time. The compiler rejects programs where types don't match — before the code runs.

## Primitives

The building blocks. Every literal has a definite type:

```basalt
fn main(stdout: Stdout) {
    let status_code = 200          // i64 — default integer type
    let temperature = 98.6         // f64 — IEEE 754 double
    let active = true              // bool
    let greeting = "hello world"   // string — immutable, UTF-8
    let nothing = nil              // nil — explicit absence

    stdout.println(status_code as string)  // Output: 200
}
```

### Integer types

| Type | Size | Range |
|------|------|-------|
| `i8` | 1 byte | -128 to 127 |
| `i16` | 2 bytes | -32,768 to 32,767 |
| `i32` | 4 bytes | -2,147,483,648 to 2,147,483,647 |
| `i64` | 8 bytes | -9.2 x 10^18 to 9.2 x 10^18 |
| `u8` | 1 byte | 0 to 255 |
| `u16` | 2 bytes | 0 to 65,535 |
| `u32` | 4 bytes | 0 to 4,294,967,295 |
| `u64` | 8 bytes | 0 to 1.8 x 10^19 |

All integer literals default to `i64`. Use a type annotation for other widths:

```basalt
fn main(stdout: Stdout) {
    let byte: u8 = 255
    let port: u16 = 8080
    let id: u64 = 0xDEADBEEF
    let mask: i32 = 0xFF
    stdout.println(byte as string)  // Output: 255
}
```

Integer literals also support hex (`0xFF`) and binary (`0b1010`) notation.

### Checked arithmetic

All integer arithmetic is checked. Overflow panics at runtime instead of silently wrapping:

```basalt
fn main(stdout: Stdout) {
    let x: u8 = 255
    let y = x + (1 as u8)
    // runtime panic: integer overflow: 255 + 1 exceeds u8 range
}
```

Division by zero and out-of-range shift amounts also panic. Use `as?` for safe narrowing conversions (see [Type Conversions](conversions.html)).

### Float

One float type: `f64` (IEEE 754 double precision, 64-bit). There is no `f32`. Float arithmetic follows IEEE 754 — no overflow panics, but division by zero produces `inf`.


## No implicit conversions

What happens if you mix integers and floats?

```basalt
fn main(stdout: Stdout) {
    let x = 1 + 2.0
}
```

> **Error:** Cannot apply operator `+` to types `i64` and `f64`.

Basalt refuses to silently convert between numeric types. You might lose precision, change sign, or overflow — and the language won't pretend that's fine.

<div class="callout callout-note"><strong>Design philosophy</strong>
Implicit conversions hide data loss. In C, <code>int x = 3.9</code> silently gives you <code>3</code>. In Basalt, every conversion is explicit and visible in the code. See <a href="conversions.html">Type Conversions</a> for the full story.
</div>

## Collections

Three built-in collection types cover most data structures:

```basalt
fn main(stdout: Stdout) {
    // Arrays: ordered, growable, homogeneous
    let temps = [72.0, 68.5, 74.1]
    stdout.println(temps[0] as string)  // Output: 72

    // Maps: ordered by insertion, key-value pairs
    let headers = {"Content-Type": "text/html", "Status": "200 OK"}
    stdout.println(headers["Status"])  // Output: 200 OK

    // Tuples: fixed-size, heterogeneous, immutable
    let user = ("Alice", 30, true)
    stdout.println(user.0)             // Output: Alice
}
```

Arrays and maps are reference types — assignment shares the same object. Use `.clone()` for an independent copy. Tuples are value types and copy on assignment.

## Optional types: absence you can see

In many languages, any reference can be null — and you don't know until it explodes at runtime. Basalt has `nil`, but it can never hide. A `string` is always a string. If a value might be absent, the type says so: `T?` (shorthand for `T | nil`) forces you to check before you use it:

```basalt
fn find_user(id: i64) -> string? {
    if id == 1 { return "Alice" }
    return nil
}

fn main(stdout: Stdout) {
    let name = find_user(42)
    // name is `string?` — you CANNOT use it as a string directly
    if name is string {
        stdout.println("Found: " + name)
    } else {
        stdout.println("Not found")  // Output: Not found
    }
}
```

The compiler forces you to handle `nil` before using the value. No null pointer exceptions. Ever.

## Result types

When an operation can fail, the return type says so: `T!E` is either a success value of type `T` or an error of type `E`. Errors are values, not exceptions thrown from unknown depths.

```basalt
fn parse_port(s: string) -> i64!string {
    if s.length == 0 { return !("empty input") }
    let n = s as? i64
    if n is nil { return !("not a number: " + s) }
    return n as i64
}
```

This is a deep topic — see [Error Handling](error-handling.html) for propagation, `guard let`, and real-world patterns.

## Union types

Sometimes a value can legitimately be one of several types. Union types make this explicit:

```basalt
fn respond(code: i64) -> i64 | string {
    if code == 200 { return "OK" }
    return code
}

fn main(stdout: Stdout) {
    let result = respond(200)
    match result {
        is string => stdout.println(result as string)
        is i64 => stdout.println("Code: " + (result as string))
    }
    // Output: OK
}
```

You can name unions with type aliases for readability:

```basalt
type JsonValue = bool | f64 | string | nil
```

The compiler requires you to handle every member of the union.

## What's Next

Now that you know what values can be, let's look at how to bind them to names. Next up: [Variables](variables.html).
