title: Type Conversions
date: 2026-03-14
description: The as and as? operators, conversion rules, and formatting behavior.

In many languages, the compiler silently converts between numeric types. A function expects `f64` and you pass an `i32` — it just works. Until the day it doesn't: an integer silently becomes a float, loses precision, and your financial calculation is wrong by a cent. Or a large integer overflows into a small one and your array index wraps around to somewhere unexpected.

Basalt requires every conversion to be explicit. You always see it. You always choose it.

## `as` — convert or panic

The `as` keyword converts a value to a target type. If the conversion is impossible, the program panics:

```basalt
fn main(stdout: Stdout) {
    // Numeric to string — always succeeds
    stdout.println(42 as string)        // Output: 42
    stdout.println(3.14 as string)      // Output: 3.14
    stdout.println(true as string)      // Output: true

    // String to number — succeeds if parseable
    let age = "29" as i64
    stdout.println(age as string)       // Output: 29

    // Float to integer — truncates toward zero
    let truncated = 3.9 as i64
    stdout.println(truncated as string) // Output: 3

    // Integer widening — always succeeds
    let big = 42 as f64
    stdout.println(big as string)       // Output: 42
}
```

When the conversion fails, you get a clear panic:

> **Panic:** cannot convert "hello" to i64

> **Panic:** value 300 out of range for u8

Use `as` when a failed conversion is a programming error — you know the value is valid, and if it isn't, something is deeply wrong.

## `as?` — convert or nil

When the input comes from the outside world — user input, file contents, configuration — failure is expected, not exceptional. Use `as?` to get an optional instead of a panic:

```basalt
fn main(stdout: Stdout) {
    let input = "not_a_number"
    let parsed = input as? i64

    if parsed is nil {
        stdout.println("invalid input")
    }
    // Output: invalid input

    let valid = "42" as? i64
    if valid is i64 {
        stdout.println("got: " + (valid as string))
    }
    // Output: got: 42
}
```

Combine with `guard let` for clean error handling (see [Error Handling](error-handling.html)):

```basalt
fn parse_port(s: string) -> i64!string {
    guard let port = s as? i64 else {
        return !("not a number: " + s)
    }
    let p = port as i64
    if p < 1 {
        return !("port must be positive")
    }
    if p > 65535 {
        return !("port out of range: " + (p as string))
    }
    return p
}
```

## Conversion table

| From | To | Behavior |
|---|---|---|
| any integer | any integer | Range-checked. Panics on overflow (`as`), nil (`as?`). |
| any integer | `f64` | Always succeeds. May lose precision for values > 2^53. |
| `f64` | any integer | Truncates toward zero. Panics on NaN, infinity, or out of range. |
| integer | `string` | Decimal digits, no leading zeros. `-7` becomes `"-7"`. |
| `f64` | `string` | Shortest decimal that round-trips. Whole numbers get one decimal: `3.0` becomes `"3.0"`, `3.14` stays `"3.14"`. |
| `bool` | `string` | `"true"` or `"false"`. |
| `string` | integer | Decimal only, leading/trailing whitespace trimmed. No hex, no underscores. |
| `string` | `f64` | Accepts `"3.14"`, `"1e10"`, `"inf"`, `"-inf"`, `"NaN"`. Whitespace trimmed. |

## Formatting details

All numeric formatting and parsing is **locale-independent**. The decimal separator is always `.`, never `,`. There are no thousands separators. `"1,000"` does not parse as an integer — it fails.

Float-to-string uses the shortest representation that round-trips back to the same `f64` value. For whole-number floats, one decimal place is always shown to distinguish from integers:

```basalt
fn main(stdout: Stdout) {
    stdout.println(3.0 as string)       // Output: 3.0  (not "3")
    stdout.println(3.14 as string)      // Output: 3.14
    stdout.println(0.1 + 0.2 as string) // Output: 0.30000000000000004
    stdout.println(1e20 as string)      // Output: 100000000000000000000.0
}
```

String-to-number parsing trims whitespace before and after, then uses strict decimal parsing. Hex (`0xFF`) and binary (`0b1010`) are valid in source code literals but are **not** accepted by `as` or `as?` from strings.

<div class="callout callout-warn"><strong>No implicit widening</strong>
Even "safe" conversions like <code>i32</code> to <code>i64</code> require an explicit <code>as</code>. Basalt treats all conversions the same — visible and intentional. This catches bugs where you accidentally mix integer widths.
</div>

## Cross-width arithmetic

You cannot mix integer sizes in arithmetic. The compiler rejects it at compile time:

```basalt
fn main(stdout: Stdout) {
    let sensor_id: i32 = 10
    let reading: i64 = 2500

    // let sum = sensor_id + reading   // COMPILE ERROR: type mismatch i32 vs i64

    let sum = (sensor_id as i64) + reading  // correct
    stdout.println(sum as string)           // Output: 2510
}
```

> **Error:** type mismatch: cannot apply + to i32 and i64

This forces you to decide which width is correct, rather than letting the compiler pick one and hoping for the best.

## Safe range checking

When you need to narrow a value — say, converting a file size from `i64` to `i32` for a legacy system — use `as?` to catch overflow:

```basalt
fn to_i32_safe(n: i64, stdout: Stdout) {
    let narrow = n as? i32
    if narrow is nil {
        stdout.println("value too large for i32: " + (n as string))
        return
    }
    stdout.println("converted: " + (narrow as string))
}

fn main(stdout: Stdout) {
    to_i32_safe(42, stdout)             // Output: converted: 42
    to_i32_safe(3000000000, stdout)     // Output: value too large for i32: 3000000000
}
```

---

You've learned the entire language. The next step is building something. The static site generator that built this documentation was written in Basalt — 500 lines of code, reading files, parsing markdown, generating HTML.
