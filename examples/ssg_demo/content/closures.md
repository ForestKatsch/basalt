title: Closures
date: 2026-03-10
description: Lambda functions and functional array methods

Closures use the `fn` keyword with inline bodies. They capture variables from their enclosing scope by reference.

```basalt
fn main(stdout: Stdout) {
    let double = fn(x: i64) -> i64 { return x * 2 }
    let square = fn(x: i64) -> i64 { return x * x }

    stdout.println(double(5) as string)  // Output: 10
    stdout.println(square(5) as string)  // Output: 25
}
```

### Capture by reference

Mutations through closures are visible in the enclosing scope:

```basalt
fn main(stdout: Stdout) {
    let mut count = 0
    let increment = fn() {
        count = count + 1
    }
    increment()
    increment()
    stdout.println(count as string)  // Output: 2
}
```

### Functional array methods

Arrays support `map`, `filter`, `find`, `any`, and `all`:

```basalt
fn main(stdout: Stdout) {
    let nums = [1, 2, 3, 4, 5]

    // map: transform each element
    let doubled = nums.map(fn(x: i64) -> i64 { return x * 2 })
    stdout.println(doubled.join(", "))  // Output: 2, 4, 6, 8, 10

    // filter: keep matching elements
    let evens = nums.filter(fn(x: i64) -> bool { return x % 2 == 0 })
    stdout.println(evens.join(", "))  // Output: 2, 4

    // any / all
    let has_big = nums.any(fn(x: i64) -> bool { return x > 3 })
    stdout.println(has_big as string)  // Output: true

    let all_pos = nums.all(fn(x: i64) -> bool { return x > 0 })
    stdout.println(all_pos as string)  // Output: true
}
```
