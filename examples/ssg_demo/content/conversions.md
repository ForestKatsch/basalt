title: Type Conversions
date: 2026-03-14
description: Type casting with as and as?

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
