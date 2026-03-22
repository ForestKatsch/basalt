title: Variables
date: 2026-03-03
description: Immutable by default. Mutable when you need it.

Most bugs from mutable state follow the same pattern: something changed a value you assumed was stable. Basalt addresses this at the language level — variables are immutable by default, and the compiler enforces it.

## Immutable by default

```basalt
fn main(stdout: Stdout) {
    let name = "config.json"
    let max_retries = 3
    stdout.println("Loading \(name), max \(max_retries as string) retries")
    // Output: Loading config.json, max 3 retries
}
```

These bindings cannot be changed. If you try:

```basalt
fn main(stdout: Stdout) {
    let name = "config.json"
    name = "settings.json"
}
```

> **Error:** Cannot assign to immutable variable `name`. Declare with `let mut` to allow reassignment.

This isn't a suggestion — it's a compile error. Your program won't build until you make your intent explicit.

## Mutable when you need it

When you genuinely need a value to change, say so with `let mut`:

```basalt
fn main(stdout: Stdout) {
    let mut attempt = 0
    let mut status = "pending"

    attempt = attempt + 1
    status = "complete"

    stdout.println("Attempt \(attempt as string): \(status)")
    // Output: Attempt 1: complete
}
```

The `mut` keyword serves as documentation. When you see `let mut` in code, you know this value will change — scan for where.

<div class="callout callout-warn"><strong>Gotcha: collections need mut too</strong>
Mutating methods like <code>push</code>, <code>sort</code>, and <code>remove</code> require the variable to be <code>mut</code>. An immutable binding to an array means you cannot modify that array at all — not just that you can't reassign the variable.

<pre><code>let items = [1, 2, 3]
items.push(4)  // Error: Cannot call mutating method on immutable variable</code></pre>

Write <code>let mut items = [1, 2, 3]</code> instead.
</div>

## Type inference

Basalt infers types from the value you assign. You don't need to write them — but you can:

```basalt
fn main(stdout: Stdout) {
    let x = 42                    // inferred: i64
    let y: f64 = 42.0            // annotated explicitly
    let byte: u8 = 255           // required: non-default integer width
    let mut m: [string: i64] = {}  // required: empty collection needs a type

    stdout.println(x as string)  // Output: 42
}
```

Annotations are required in two cases: when you want a non-default numeric type (like `u8` instead of `i64`), and when the compiler can't infer the type (like an empty map or array).

## Shadowing

You can declare a new variable with the same name in the same scope. This **shadows** the previous binding — it doesn't mutate it:

```basalt
fn main(stdout: Stdout) {
    let input = "  42  "
    let input = input.trim()        // shadows: now "42"
    let input = input as? i64       // shadows: now i64?

    if input is i64 {
        stdout.println("Parsed: " + (input as string))
        // Output: Parsed: 42
    }
}
```

Each `let` creates a new binding. The old value is untouched — if anything else referenced it, they still see the original. Shadowing is useful for transforming a value through a pipeline without inventing new names for each step.

This is different from mutation. With `let mut`, you change what a name points to. With shadowing, you create an entirely new binding that happens to reuse the name — and you can even change the type, as the example above shows.

## What's Next

Variables hold values. Functions transform them. Next up: [Functions](functions.html).
