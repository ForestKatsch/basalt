title: Closures
date: 2026-03-10
description: Lambda syntax, capture by reference, and functional array methods.

A closure is a function defined inline that can capture variables from its surrounding scope. You write them with the `fn` keyword, just like regular functions, but without a name.

## Basic closures

```basalt
fn main(stdout: Stdout) {
    let celsius_to_fahrenheit = fn(c: f64) -> f64 { return c * 9.0 / 5.0 + 32.0 }

    stdout.println(celsius_to_fahrenheit(0.0) as string)    // Output: 32
    stdout.println(celsius_to_fahrenheit(100.0) as string)  // Output: 212
}
```

Closures have the same syntax as functions — parameters, return type, body — but they are values. You can assign them, pass them, and return them.

## Capture by reference

Closures see the variables from their enclosing scope. Changes go both ways — the closure can read and mutate captured variables, and those mutations are visible outside:

```basalt
fn main(stdout: Stdout) {
    let mut total = 0
    let add = fn(amount: i64) {
        total = total + amount
    }

    add(10)
    add(25)
    add(7)
    stdout.println(total as string)  // Output: 42
}
```

This is capture by reference, not by copy. The closure and the enclosing scope share the same variable.

## Array methods

Closures are used heavily with array methods: `map`, `filter`, `find`, `any`, and `all`. These take a closure and apply it to each element.

```basalt
fn main(stdout: Stdout) {
    let temperatures = [22.5, 18.0, 31.2, 15.8, 27.6]

    let hot_days = temperatures.filter(fn(t: f64) -> bool { return t > 25.0 })
    stdout.println(hot_days.length as string)  // Output: 2

    let in_fahrenheit = temperatures.map(fn(c: f64) -> f64 { return c * 9.0 / 5.0 + 32.0 })
    stdout.println(in_fahrenheit[0] as string)  // Output: 72.5

    let any_freezing = temperatures.any(fn(t: f64) -> bool { return t <= 0.0 })
    stdout.println(any_freezing as string)  // Output: false

    let all_positive = temperatures.all(fn(t: f64) -> bool { return t > 0.0 })
    stdout.println(all_positive as string)  // Output: true
}
```

## Pipelines

Chain methods to build data-processing pipelines. Each step transforms the data and feeds into the next:

```basalt
fn main(stdout: Stdout) {
    let scores = [85, 42, 91, 67, 73, 55, 98, 30]

    let result = scores
        .filter(fn(s: i64) -> bool { return s >= 60 })
        .map(fn(s: i64) -> string {
            if s >= 90 { return (s as string) + " (A)" }
            if s >= 80 { return (s as string) + " (B)" }
            if s >= 70 { return (s as string) + " (C)" }
            return (s as string) + " (D)"
        })
        .join(", ")

    stdout.println(result)
    // Output: 85 (B), 91 (A), 67 (D), 73 (C), 98 (A)
}
```

Filter keeps only passing scores, map formats each with its grade, join produces the final string.

## Finding elements

`find` returns the first element matching a predicate, as an optional:

```basalt
fn main(stdout: Stdout) {
    let names = ["Alice", "Bob", "Charlie", "Diana"]

    let found = names.find(fn(n: string) -> bool { return n.starts_with("C") })
    if found is string {
        stdout.println("Found: " + (found as string))
    }
    // Output: Found: Charlie
}
```

## Closures as strategy

<div class="callout callout-tip"><strong>Try this</strong>
Closures let you swap behavior without building a class hierarchy. Pass different closures to the same function to get different behavior — the strategy pattern without the ceremony.
</div>

```basalt
fn apply_discount(prices: [f64], strategy: fn(f64) -> f64) -> [f64] {
    return prices.map(strategy)
}

fn main(stdout: Stdout) {
    let prices = [29.99, 49.99, 99.99]

    let half_off = fn(p: f64) -> f64 { return p * 0.5 }
    let ten_off = fn(p: f64) -> f64 { return p - 10.0 }

    let sale = apply_discount(prices, half_off)
    stdout.println(sale[0] as string)  // Output: 14.995

    let coupon = apply_discount(prices, ten_off)
    stdout.println(coupon[0] as string)  // Output: 19.99
}
```

Functions that accept closures as parameters declare the type as `fn(ArgTypes) -> ReturnType`.

## Type narrowing and closures

A mutable variable captured by a closure cannot be type-narrowed with `is`. The closure could mutate the variable between the type check and its use:

```basalt
fn main(stdout: Stdout) {
    let mut val: i64 | string = "hello"
    let swap = fn() { val = 42 }

    if val is string {
        // val could have been changed by swap() between the check and here
        stdout.println(val)  // ERROR
    }
}
```

> **Error:** Cannot narrow `val`: mutable variable is captured by a closure.

Narrowing still works for:
- Immutable variables (even if captured)
- Mutable variables not captured by any closure

## What's Next

You now have functions, closures, structs, and enums. [Modules](modules.html) show how to organize all of this across multiple files.
