title: Pattern Matching
date: 2026-03-06
description: The compiler checks every case. Forget one, it tells you.

Pattern matching in Basalt isn't syntactic sugar — it's a safety mechanism. The `match` expression checks a value against patterns, and the compiler **guarantees you've handled every possibility**. If you add a variant to an enum and forget to update a `match`, the compiler stops you before the code ships.

## Matching literals

At its simplest, `match` replaces chains of `if`/`else if`:

```basalt
fn describe_status(code: i64) -> string {
    match code {
        200 => return "OK"
        301 => return "Moved Permanently"
        404 => return "Not Found"
        500 => return "Internal Server Error"
        _ => return "Unknown status"
    }
    return ""
}

fn main(stdout: Stdout) {
    stdout.println(describe_status(404))  // Output: Not Found
    stdout.println(describe_status(418))  // Output: Unknown status
}
```

The `_` wildcard catches everything not explicitly listed. Without it, the compiler would reject this `match` — an `i64` has too many values to enumerate.

## Matching enum variants

This is where `match` truly shines. Define an enum with associated data, then destructure it:

```basalt
type HttpBody {
    Text(string)
    Json(string)
    Empty
}

fn content_type(body: HttpBody) -> string {
    match body {
        HttpBody.Text(_) => return "text/plain"
        HttpBody.Json(_) => return "application/json"
        HttpBody.Empty => return "none"
    }
    return ""
}

fn main(stdout: Stdout) {
    let body = HttpBody.Json("{\"ok\": true}")
    stdout.println(content_type(body))  // Output: application/json
}
```

Each pattern destructures the variant, binding inner values to names you can use in the body. Use `_` when you don't need the inner value.

## Exhaustiveness: the safety net

Leave out a variant and the compiler tells you:

```basalt
fn content_type(body: HttpBody) -> string {
    match body {
        HttpBody.Text(_) => return "text/plain"
        HttpBody.Json(_) => return "application/json"
    }
    return ""
}
```

> **Error:** Non-exhaustive match. Missing pattern: `HttpBody.Empty`.

<div class="callout callout-note"><strong>Why this matters</strong>
Imagine you add a <code>Binary(bytes)</code> variant to <code>HttpBody</code> six months from now. Without exhaustiveness checking, every <code>match</code> on <code>HttpBody</code> silently does the wrong thing. With it, the compiler finds every location that needs updating — instantly. This turns a runtime bug into a compile-time checklist.
</div>

## Type narrowing with `is`

For union types, `is` patterns narrow the type within each branch:

```basalt
fn format_cell(val: i64 | f64 | string | bool) -> string {
    match val {
        is i64 => return (val as string)
        is f64 => return (val as string)
        is string => return "\"" + val + "\""
        is bool => return if val as bool { "yes" } else { "no" }
    }
    return ""
}

fn main(stdout: Stdout) {
    stdout.println(format_cell(42))        // Output: 42
    stdout.println(format_cell("hello"))   // Output: "hello"
    stdout.println(format_cell(true))      // Output: yes
}
```

Inside each `is` branch, the compiler knows the exact type — you can use type-specific operations without casting.

## Matching results

`match` handles result types cleanly. The `!err` pattern matches the error case:

```basalt
fn divide(a: f64, b: f64) -> f64!string {
    if b == 0.0 { return !("division by zero") }
    return a / b
}

fn main(stdout: Stdout) {
    match divide(10.0, 0.0) {
        !err => stdout.println("Error: " + err)
        val => stdout.println("Result: " + (val as string))
    }
    // Output: Error: division by zero

    match divide(10.0, 3.0) {
        !err => stdout.println("Error: " + err)
        val => stdout.println("Result: " + (val as string))
    }
    // Output: Result: 3.3333333333333335
}
```

Both branches must be present — you can't ignore the error case. For lighter-weight error handling, see the `?` operator and `guard let` in [Error Handling](error-handling.html).

<div class="callout callout-tip"><strong>Try this</strong>
Define a <code>type Shape { Circle(f64), Rect(f64, f64), Triangle(f64, f64, f64) }</code> and write an <code>area</code> function using <code>match</code>. Then add a <code>Point</code> variant with no data and see what the compiler tells you.
</div>

## What's Next

Pattern matching forces you to handle every case — including errors. But what's the best way to structure error handling across a whole program? Next up: [Error Handling](error-handling.html).
