title: Pattern Matching
date: 2026-03-06
description: Exhaustive pattern matching with match expressions

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
