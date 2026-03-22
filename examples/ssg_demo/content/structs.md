title: Structs
date: 2026-03-08
description: Defining structs, methods, reference semantics, and cloning.

Structs group related data under a name. You define them with the `type` keyword and construct them with field initializers.

## Defining and using structs

```basalt
type User {
    name: string
    email: string
    login_count: i64
}

fn main(stdout: Stdout) {
    let alice = User {
        name: "Alice Chen",
        email: "alice@example.com",
        login_count: 0,
    }
    stdout.println(alice.name)   // Output: Alice Chen
    stdout.println(alice.email)  // Output: alice@example.com
}
```

Every field must be set at construction. There are no default values — the compiler rejects partial initialization.

## Methods

Methods are functions defined inside the type body. Instance methods take `self: Self` as their first parameter. Functions without `self` are static — called on the type, not an instance.

```basalt
import "std/math"

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

    fn translate(self: Self, dx: f64, dy: f64) -> Point {
        return Point { x: self.x + dx, y: self.y + dy }
    }
}

fn main(stdout: Stdout) {
    let a = Point.origin()
    let b = Point { x: 3.0, y: 4.0 }
    stdout.println(a.distance(b) as string)  // Output: 5

    let c = b.translate(1.0, -1.0)
    stdout.println(c.x as string)  // Output: 4
    stdout.println(c.y as string)  // Output: 3
}
```

Static methods like `origin()` are called on the type name. Instance methods like `distance()` are called on a value.

## Reference semantics

Structs are reference types. Assignment shares the object, not copies it:

```basalt
fn main(stdout: Stdout) {
    let mut point = Point { x: 1.0, y: 2.0 }
    let mut alias = point

    alias.x = 99.0
    stdout.println(point.x as string)  // Output: 99
}
```


<div class="callout callout-warn"><strong>Reference semantics apply everywhere</strong>
Mutating through one variable is visible through every other variable that refers to the same struct. This applies to function arguments too — if you pass a struct to a function, the function can see mutations you make later.
</div>

## Cloning for independence

For an independent copy, call `clone()`:

```basalt
fn main(stdout: Stdout) {
    let mut original = Point { x: 5.0, y: 10.0 }
    let mut copy = original.clone()

    copy.x = 0.0
    stdout.println(original.x as string)  // Output: 5 (unaffected)
    stdout.println(copy.x as string)      // Output: 0
}
```

`clone()` performs a deep copy. Changes to the clone never affect the original, and vice versa. Use it whenever you need to hand off data that the caller might mutate.

## Passing structs to functions

Because structs are reference types, passing them to functions doesn't copy anything. This is efficient — but it means the function operates on the same object:

```basalt
fn reset_origin(p: Point) {
    p.x = 0.0
    p.y = 0.0
}

fn main(stdout: Stdout) {
    let mut pos = Point { x: 7.0, y: 3.0 }
    reset_origin(pos)
    stdout.println(pos.x as string)  // Output: 0
}
```

If you want the function to work on its own copy, clone before passing — or clone inside the function and return the new value.

## Structs with struct fields

Structs can contain other structs. The same reference semantics apply to nested structs:

```basalt
type Segment {
    start: Point
    end: Point

    fn length(self: Self) -> f64 {
        return self.start.distance(self.end)
    }
}

fn main(stdout: Stdout) {
    let seg = Segment {
        start: Point { x: 0.0, y: 0.0 },
        end: Point { x: 3.0, y: 4.0 },
    }
    stdout.println(seg.length() as string)  // Output: 5
}
```

## What's Next

Structs give you one shape of data. [Enums](enums.html) give you a choice between shapes — a value that could be one of several distinct variants, each with its own data.
