title: Variables
date: 2026-03-03
description: Variables, mutability, and type inference

Variables are immutable by default:

```basalt
fn main(stdout: Stdout) {
    let name = "Basalt"
    let age = 1
    stdout.println("\(name) is \(age) year old")
    // Output: Basalt is 1 year old
}
```

Use `mut` when you need to reassign or mutate:

```basalt
fn main(stdout: Stdout) {
    let mut count = 0
    count = count + 1
    stdout.println(count as string)  // Output: 1

    // mut is also required for mutating methods on collections
    let mut items = [1, 2, 3]
    items.push(4)
    stdout.println(items.length as string)  // Output: 4
}
```

Type inference works for most declarations. Add annotations when needed:

```basalt
fn main(stdout: Stdout) {
    let x = 42                  // inferred as i64
    let y: f64 = 42.0           // annotated
    let byte: u8 = 255          // required for non-default integer widths
    let mut m: [string: i64] = {}  // required for empty collections

    stdout.println(x as string)
}
```
