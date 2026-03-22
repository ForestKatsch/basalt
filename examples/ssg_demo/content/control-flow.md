title: Control Flow
date: 2026-03-05
description: Conditions are bool. Loops are clear. Guards catch problems early.

Basalt's control flow has one principle: **say what you mean**. Conditions must be booleans — no truthy/falsy surprises. Loops iterate over real collections. Guards push error handling to the top so the happy path reads clean.

## No truthiness

In JavaScript, `if (0)`, `if ("")`, and `if (null)` are all falsy. This leads to bugs where empty strings and zero are accidentally treated as "missing." Basalt rejects this entirely:

```basalt
fn main(stdout: Stdout) {
    let count = 0
    if count { stdout.println("truthy") }
}
```

> **Error:** Condition must be `bool`, found `i64`.

Write what you mean: `if count > 0 { ... }` or `if count != 0 { ... }`.

## if/else as expressions

`if` produces a value. You can bind it directly to a variable:

```basalt
fn main(stdout: Stdout) {
    let temp = 72
    let label = if temp > 80 { "hot" } else { "comfortable" }
    stdout.println(label)  // Output: comfortable

    let grade = if temp >= 90 { "A" }
        else if temp >= 80 { "B" }
        else if temp >= 70 { "C" }
        else { "F" }
    stdout.println(grade)  // Output: C
}
```

Both branches must produce the same type. If they don't, the compiler tells you:

> **Error:** `if` branch returns `string` but `else` branch returns `i64`.

## for-in loops

`for` iterates over arrays, maps, strings, and ranges. No index variables, no off-by-one errors:

```basalt
fn main(stdout: Stdout) {
    // Arrays
    let files = ["main.bas", "utils.bas", "test.bas"]
    for file in files {
        stdout.println("Compiling \(file)")
    }

    // With index — value first, index second
    for file, i in files {
        stdout.println("\(i as string): \(file)")
    }

    // Maps — key first, value second
    let env = {"HOME": "/Users/forest", "LANG": "en_US"}
    for key, value in env {
        stdout.println("\(key)=\(value)")
    }

    // Ranges — exclusive end
    let mut total = 0
    for i in 1..6 {
        total = total + i
    }
    stdout.println(total as string)  // Output: 15

    // Strings — iterates by character
    for ch in "OK!" {
        stdout.print("[" + ch + "]")
    }
    stdout.println("")  // Output: [O][K][!]
}
```

## while and loop

`while` runs as long as a condition holds. `loop` runs forever until you `break`:

```basalt
fn main(stdout: Stdout) {
    // while
    let mut retries = 3
    while retries > 0 {
        stdout.println("Attempt \(retries as string)")
        retries = retries - 1
    }

    // loop with break
    let mut n = 1
    loop {
        if n > 100 { break }
        n = n * 2
    }
    stdout.println(n as string)  // Output: 128

    // continue skips the current iteration
    for i in 0..10 {
        if i % 3 == 0 { continue }
        stdout.print("\(i as string) ")
    }
    stdout.println("")  // Output: 1 2 4 5 7 8
}
```

## guard: assert and bail

`guard` asserts a condition at the top of a function. If the condition is false, the `else` block must diverge — `return`, `break`, `continue`, or `panic`. This pushes failure handling upward so the rest of the function can assume success:

```basalt
fn process_age(input: string, stdout: Stdout) {
    guard input.length > 0 else {
        stdout.println("Error: empty input")
        return
    }

    let parsed = input as? i64
    guard parsed is i64 else {
        stdout.println("Error: not a number")
        return
    }

    // Happy path — no nesting
    stdout.println("Age: " + (parsed as string))
}

fn main(stdout: Stdout) {
    process_age("25", stdout)   // Output: Age: 25
    process_age("", stdout)     // Output: Error: empty input
    process_age("abc", stdout)  // Output: Error: not a number
}
```

## guard let: unwrap or bail

`guard let` combines unwrapping with early return. It extracts the success value from an optional or result into the **enclosing scope** — not a nested block:

```basalt
fn load_config(fs: Fs, stdout: Stdout) {
    guard let content = fs.read_file("config.txt") else {
        stdout.println("Could not read config")
        return
    }
    // content is `string` here, not a result type
    stdout.println("Loaded \(content.length as string) bytes")
}
```

Compare this to nested `if` or `match` — `guard let` keeps the happy path at the top indentation level. You'll see it everywhere once you start writing real Basalt. It's especially powerful with [Error Handling](error-handling.html).

<div class="callout callout-tip"><strong>Try this</strong>
Write a function that takes an array of strings and returns the first one that parses as an integer, using <code>guard let</code> inside a <code>for</code> loop with <code>continue</code> in the else block.
</div>

## What's Next

`if` and `guard` handle simple conditions. When you need to match against multiple patterns at once — especially enum variants and union types — you want something more powerful. Next up: [Pattern Matching](pattern-matching.html).
