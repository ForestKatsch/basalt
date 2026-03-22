title: Functions
date: 2026-03-04
description: Everything is explicit. Parameters, return types, return statements.

All parameters require type annotations. Return type is required for non-void functions. Return is always explicit.

## Declaring functions

```basalt
fn celsius_to_fahrenheit(c: f64) -> f64 {
    return c * 1.8 + 32.0
}

fn main(stdout: Stdout) {
    let temp = celsius_to_fahrenheit(100.0)
    stdout.println(temp as string)  // Output: 212
}
```

Parameters require type annotations — always. The return type follows `->`. If a function returns nothing (only performs side effects), omit the return type:

```basalt
fn log_request(method: string, path: string, stdout: Stdout) {
    stdout.println("\(method) \(path)")
}
```

## Explicit return

Basalt requires you to write `return`. The last expression in a function body is **not** implicitly returned.

<div class="callout callout-note"><strong>Why explicit return?</strong>
In languages with implicit return, adding a logging line at the end of a function silently changes its return value to <code>nil</code>. Basalt avoids this class of bug entirely. If a function promises to return a value, the compiler verifies every code path has an explicit <code>return</code>.
</div>

If you declare a return type but forget to return:

```basalt
fn double(x: i64) -> i64 {
    let result = x * 2
}
```

> **Error:** Function `double` declares return type `i64` but not all code paths return a value.

And if you return the wrong type:

```basalt
fn label(code: i64) -> string {
    return code
}
```

> **Error:** Cannot return `i64` from function with return type `string`.

No ambiguity. The compiler catches it, points at the line, and tells you what's wrong.

## Multiple parameters and early return

Functions can return early from any point. This keeps the happy path unindented:

```basalt
fn http_status(code: i64) -> string {
    if code < 100 { return "invalid" }
    if code < 200 { return "informational" }
    if code < 300 { return "success" }
    if code < 400 { return "redirect" }
    if code < 500 { return "client error" }
    return "server error"
}

fn main(stdout: Stdout) {
    stdout.println(http_status(404))  // Output: client error
    stdout.println(http_status(200))  // Output: success
}
```

## Functions as first-class values

Functions are values. You can pass them as arguments, store them in variables, and return them from other functions:

```basalt
fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
    return f(x)
}

fn double(x: i64) -> i64 {
    return x * 2
}

fn negate(x: i64) -> i64 {
    return 0 - x
}

fn main(stdout: Stdout) {
    stdout.println(apply(double, 5) as string)   // Output: 10
    stdout.println(apply(negate, 3) as string)    // Output: -3
}
```

The type `fn(i64) -> i64` describes any function that takes one `i64` and returns an `i64`. This enables higher-order patterns — mapping, filtering, and composing operations — without any special syntax. We'll see this in action with [Closures](closures.html).

## Forward references

Functions can call other functions defined later in the same file. All function signatures are registered before any bodies are checked:

```basalt
fn main(stdout: Stdout) {
    stdout.println(greet("world"))  // OK — greet is defined below
}

fn greet(name: string) -> string {
    return "Hello, " + name + "!"
}
```

## What's Next

Functions describe *what* to compute. Control flow describes *when* and *how*. Next up: [Control Flow](control-flow.html).
