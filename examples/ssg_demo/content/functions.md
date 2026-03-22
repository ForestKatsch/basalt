title: Functions
date: 2026-03-04
description: Function declarations and first-class functions

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
