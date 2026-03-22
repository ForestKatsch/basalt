title: Structs
date: 2026-03-08
description: Struct types, methods, and reference semantics

Define structs with the `type` keyword. Structs are reference types.

```basalt
type Point {
    x: f64
    y: f64
}

fn main(stdout: Stdout) {
    let p = Point { x: 3.0, y: 4.0 }
    stdout.println(p.x as string)  // Output: 3
    stdout.println(p.y as string)  // Output: 4
}
```

### Methods

Instance methods take `self: Self` as their first parameter. Static methods do not.

```basalt
type Point {
    x: f64
    y: f64

    fn origin() -> Point {
        return Point { x: 0.0, y: 0.0 }
    }

    fn distance(self: Self, other: Point) -> f64 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        return math.sqrt(dx * dx + dy * dy)
    }
}

fn main(stdout: Stdout) {
    let a = Point.origin()
    let b = Point { x: 3.0, y: 4.0 }
    stdout.println(a.distance(b) as string)  // Output: 5
}
```

### Reference semantics

```basalt
fn main(stdout: Stdout) {
    let mut p = Point { x: 1.0, y: 2.0 }
    let mut q = p        // q and p refer to the same struct
    q.x = 10.0
    stdout.println(p.x as string)  // Output: 10 (same object)

    let mut r = p.clone()  // independent copy
    r.x = 99.0
    stdout.println(p.x as string)  // Output: 10 (unaffected)
}
```
