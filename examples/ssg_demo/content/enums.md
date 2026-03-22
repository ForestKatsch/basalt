title: Enums
date: 2026-03-09
description: A closed set of possibilities. The compiler ensures you handle each one.

An enum defines a type that can be exactly one of several variants. Unlike structs, which always have the same fields, an enum value is one possibility from a fixed set.

## Simple enums

At their simplest, enums are named constants:

```basalt
type Direction { North, South, East, West }

fn describe(d: Direction) -> string {
    match d {
        Direction.North => return "heading north"
        Direction.South => return "heading south"
        Direction.East  => return "heading east"
        Direction.West  => return "heading west"
    }
    return ""
}

fn main(stdout: Stdout) {
    let heading = Direction.North
    stdout.println(describe(heading))  // Output: heading north
}
```

Variants are always accessed through the type name: `Direction.North`, never just `North`.

## Data-carrying variants

Each variant can carry its own data. Different variants can carry different types and different numbers of fields:

```basalt
type Shape {
    Circle(f64)
    Rect(f64, f64)
    Triangle(f64, f64, f64)
}

fn area(s: Shape) -> f64 {
    match s {
        Shape.Circle(r) => return 3.14159 * r * r
        Shape.Rect(w, h) => return w * h
        Shape.Triangle(a, b, h) => return 0.5 * b * h
    }
    return 0.0
}

fn main(stdout: Stdout) {
    let shapes = [
        Shape.Circle(5.0),
        Shape.Rect(3.0, 4.0),
    ]
    for shape in shapes {
        stdout.println(area(shape) as string)
    }
    // Output: 78.53975
    // Output: 12
}
```

The exhaustiveness guarantee from [Pattern Matching](pattern-matching.html) makes enums powerful — add a new variant and the compiler tells you every place that needs updating.

## Testing variants with `is`

When you need to check which variant you have without destructuring:

```basalt
type Status { Active, Suspended, Deleted }

fn main(stdout: Stdout) {
    let user_status = Status.Active

    if user_status is Status.Active {
        stdout.println("user can log in")
    }
    // Output: user can log in
}
```

Use `is` for branching, `match` for extracting data.

## Modeling real domains

Enums shine when you model domains where something can be one of several distinct states. Here's a JSON value type:

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

Variants can reference the enclosing type, so recursive structures like trees and nested documents work naturally. Every function that receives a `Json` value must handle every variant — the compiler ensures nothing is missed.

## State machines

Enums are the natural fit for state machines. Each state is a variant, and transitions are functions that return the next state:

```basalt
type ConnectionState {
    Disconnected
    Connecting(string)
    Connected(string, i64)
    Failed(string)
}

fn describe_state(state: ConnectionState) -> string {
    match state {
        ConnectionState.Disconnected => return "not connected"
        ConnectionState.Connecting(host) => return "connecting to " + host
        ConnectionState.Connected(host, port) => {
            return "connected to " + host + ":" + (port as string)
        }
        ConnectionState.Failed(reason) => return "failed: " + reason
    }
    return ""
}

fn main(stdout: Stdout) {
    let state = ConnectionState.Connecting("api.example.com")
    stdout.println(describe_state(state))
    // Output: connecting to api.example.com

    let next = ConnectionState.Connected("api.example.com", 443)
    stdout.println(describe_state(next))
    // Output: connected to api.example.com:443
}
```

<div class="callout callout-note"><strong>Enums vs. type unions</strong>
Basalt also has union types like <code>string | i64</code> for ad-hoc combinations. Enums are different — they define a <em>named, closed</em> set of possibilities with meaningful variant names. Use unions when combining existing types; use enums when defining a domain concept.
</div>

## What's Next

Enums and match give you powerful data modeling. [Closures](closures.html) add another dimension — functions you can create on the fly and pass around as values.
