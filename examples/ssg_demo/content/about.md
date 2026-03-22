title: About Basalt
date: 2026-03-20
description: Learn about the Basalt programming language

# About Basalt

Basalt was designed with a single principle: **no surprises**.

## Type System

Every value has a type known at compile time:

- `i64`, `f64`, `bool`, `string` — primitives
- `[T]` — arrays, `[K: V]` — maps
- `T?` — optionals (no null)
- `T!E` — results (no exceptions)

### Example

Here is a simple function:

```
fn greet(name: string) -> string {
    return "Hello, " + name + "!"
}
```

The compiler catches type errors *before* your code runs.

---

*Built with Basalt's static site generator.*
