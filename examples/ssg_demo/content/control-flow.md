title: Control Flow
date: 2026-03-05
description: Conditionals, loops, guards, and pattern matching

### if/else

Conditions must be `bool`. There is no truthiness — `if 0` and `if ""` are compile errors.

`if` is an expression that produces a value:

```basalt
fn main(stdout: Stdout) {
    let x = 10
    let label = if x > 0 { "positive" } else { "non-positive" }
    stdout.println(label)  // Output: positive

    let grade = if x >= 90 { "A" }
        else if x >= 80 { "B" }
        else if x >= 70 { "C" }
        else { "F" }
    stdout.println(grade)  // Output: F
}
```

### while

```basalt
fn main(stdout: Stdout) {
    let mut i = 1
    while i <= 5 {
        stdout.print(i as string + " ")
        i = i + 1
    }
    stdout.println("")
    // Output: 1 2 3 4 5
}
```

### for-in

Iterate over arrays, maps, strings, and ranges:

```basalt
fn main(stdout: Stdout) {
    // Array iteration
    let fruits = ["apple", "banana", "cherry"]
    for fruit in fruits {
        stdout.println(fruit)
    }

    // With index (value first, index second)
    for fruit, i in fruits {
        stdout.println("\(i as string): \(fruit)")
    }

    // Map iteration (key first, value second)
    let ages = {"alice": 30, "bob": 25}
    for name, age in ages {
        stdout.println("\(name) is \(age as string)")
    }

    // Range (exclusive end)
    let mut sum = 0
    for i in 0..10 {
        sum = sum + i
    }
    stdout.println(sum as string)  // Output: 45

    // String iteration (by character)
    for ch in "hi!" {
        stdout.print("[" + ch + "]")
    }
    stdout.println("")  // Output: [h][i][!]
}
```

### loop, break, continue

```basalt
fn main(stdout: Stdout) {
    // Infinite loop with break
    let mut n = 0
    loop {
        n = n + 1
        if n > 5 { break }
    }
    stdout.println(n as string)  // Output: 6

    // Skip odd numbers with continue
    let mut evens = 0
    for i in 0..10 {
        if i % 2 != 0 { continue }
        evens = evens + 1
    }
    stdout.println(evens as string)  // Output: 5
}
```

### guard / guard let

`guard` asserts a condition. If it fails, the `else` block must diverge (`return`, `break`, `continue`, or `panic`).

```basalt
fn process(x: i64, stdout: Stdout) {
    guard x > 0 else {
        stdout.println("must be positive")
        return
    }
    stdout.println("processing: " + (x as string))
}

fn main(stdout: Stdout) {
    process(5, stdout)   // Output: processing: 5
    process(-1, stdout)  // Output: must be positive
}
```

`guard let` unwraps optionals and results into the enclosing scope:

```basalt
fn load_config(fs: Fs, stdout: Stdout) {
    guard let content = fs.read_file("config.txt") else {
        stdout.println("Could not read config")
        return
    }
    // content is `string` here, not a result type
    stdout.println("Config loaded: " + (content.length as string) + " chars")
}
```
