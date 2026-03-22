title: Enums
date: 2026-03-09
description: Enum types with data-carrying variants

Enums define a closed set of variants. Variants can carry data.

```basalt
type Color { Red, Green, Blue }

type Option {
    Some(i64)
    None
}

fn main(stdout: Stdout) {
    let c = Color.Red

    let x = Option.Some(42)
    match x {
        Option.Some(val) => stdout.println(val as string)
        Option.None => stdout.println("none")
    }
    // Output: 42
}
```

### Data-carrying variants

```basalt
type Shape {
    Circle(f64)
    Rect(f64, f64)
}

fn describe(s: Shape) -> string {
    match s {
        Shape.Circle(r) => return "circle with radius " + (r as string)
        Shape.Rect(w, h) => return (w as string) + "x" + (h as string) + " rectangle"
    }
    return ""
}

fn main(stdout: Stdout) {
    stdout.println(describe(Shape.Circle(5.0)))
    // Output: circle with radius 5
    stdout.println(describe(Shape.Rect(3.0, 4.0)))
    // Output: 3x4 rectangle
}
```

### Testing variants with is

```basalt
fn main(stdout: Stdout) {
    let x = Option.Some(42)
    if x is Option.Some {
        stdout.println("has a value")
    }
    // Use match to destructure the inner data
}
```

### Recursive enums

Enum variants can reference the enclosing type, enabling trees and nested data:

```basalt
type Json {
    Null
    Bool(bool)
    Num(f64)
    Str(string)
    Arr([Json])
    Obj([string: Json])
}
```
