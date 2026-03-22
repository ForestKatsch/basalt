title: Error Handling
date: 2026-03-07
description: No exceptions. No surprises. Errors are values you can see in the type.

If a function can fail, its return type says so. `T!E` holds either a success value of type `T` or an error of type `E`. The compiler ensures every result is handled.

## Result types

`T!E` is a type that holds either a success value of type `T` or an error of type `E`:

```basalt
fn parse_temperature(s: string) -> f64!string {
    let trimmed = s.trim()
    if trimmed.length == 0 {
        return !("empty input")
    }
    let n = trimmed as? f64
    if n is nil {
        return !("not a number: " + trimmed)
    }
    return n as f64
}
```

Success values return normally. Errors are returned with `!(value)`. The caller sees both possibilities in the type — nothing is hidden.

## Handling results with match

The most explicit way to handle a result is `match`:

```basalt
fn main(stdout: Stdout) {
    match parse_temperature("72.5") {
        !err => stdout.println("Failed: " + err)
        temp => stdout.println("Temperature: " + (temp as string))
    }
    // Output: Temperature: 72.5

    match parse_temperature("warm") {
        !err => stdout.println("Failed: " + err)
        temp => stdout.println("Temperature: " + (temp as string))
    }
    // Output: Failed: not a number: warm
}
```

Both arms are required. You cannot ignore the error case — the compiler enforces this.

## Propagating with ?

When your function also returns a result, `?` propagates errors automatically. If the value is an error, it returns from the enclosing function immediately. If it's a success, it unwraps the value:

```basalt
fn load_temperature(fs: Fs) -> f64!string {
    let content = fs.read_file("temperature.txt")?
    let temp = parse_temperature(content)?
    return temp
}
```

Two operations, two possible failure points, zero nesting. Compare this to the equivalent nested `match`:

```basalt
fn load_temperature_verbose(fs: Fs) -> f64!string {
    match fs.read_file("temperature.txt") {
        !err => return !(err)
        content => {
            match parse_temperature(content) {
                !err => return !(err)
                temp => return temp
            }
        }
    }
    return !("unreachable")
}
```

The `?` version says the same thing in three lines instead of eleven. The enclosing function must return a compatible result type — the compiler checks this.

## guard let: unwrap or bail

When you're in a function that doesn't return a result (like `main`), use `guard let` to unwrap and handle the error in place:

```basalt
fn main(stdout: Stdout, fs: Fs) {
    guard let content = fs.read_file("config.txt") else {
        stdout.println("Could not read config file")
        return
    }
    // content is `string` here — unwrapped and safe
    stdout.println("Config: " + content)
}
```

The variable `content` is available in the **enclosing scope**, not just inside a block. The `else` branch must diverge — `return`, `break`, or `panic`.

## A real example: read, parse, validate

Here's how these pieces compose in practice:

```basalt
fn load_user_age(fs: Fs) -> i64!string {
    let content = fs.read_file("user_age.txt")?
    let trimmed = content.trim()
    let n = trimmed as? i64
    if n is nil {
        return !("invalid age: " + trimmed)
    }
    let age = n as i64
    if age < 0 { return !("age cannot be negative") }
    if age > 150 { return !("age seems unrealistic: " + (age as string)) }
    return age
}

fn main(stdout: Stdout, fs: Fs) {
    match load_user_age(fs) {
        !err => stdout.println("Error: " + err)
        age => stdout.println("User age: " + (age as string))
    }
}
```

Each possible failure — file not found, not a number, out of range — is explicit. The caller sees a single `i64!string` and decides how to handle failure. No exception hierarchy to memorize. No hidden control flow.

## Custom error types

The error type in `T!E` can be any type — not just `string`. For real applications, define an enum:

```basalt
type ConfigError {
    FileNotFound(string)
    ParseFailed(string)
    MissingField(string)
}

fn load_config(fs: Fs) -> [string: string]!ConfigError {
    guard let raw = fs.read_file("config.txt") else {
        return !(ConfigError.FileNotFound("config.txt"))
    }
    let mut config: [string: string] = {}
    let lines = raw.split("\n")
    for line in lines {
        let sep = line.index_of("=")
        if sep < 0 { continue }
        let key = line.slice(0, sep).trim()
        let value = line.slice(sep + 1, line.length).trim()
        config[key] = value
    }
    if !config.contains_key("name") {
        return !(ConfigError.MissingField("name"))
    }
    return config
}

fn main(stdout: Stdout, fs: Fs) {
    match load_config(fs) {
        !err => {
            match err {
                ConfigError.FileNotFound(path) => stdout.println("File not found: " + path)
                ConfigError.ParseFailed(msg) => stdout.println("Parse error: " + msg)
                ConfigError.MissingField(name) => stdout.println("Missing: " + name)
            }
        }
        config => stdout.println("Loaded config: " + config["name"])
    }
}
```

Using an enum for errors gives you exhaustive handling — the compiler ensures you handle `FileNotFound`, `ParseFailed`, AND `MissingField`. If you add a new error variant later, every `match` that handles the error type tells you about the missing case.

## What happens if you ignore a result?

You can't. If a function returns `T!E` and you don't handle the error, the compiler tells you:

> **Error:** Result type `i64!string` must be handled. Use `match`, `?`, or `guard let`.

This is the core guarantee: errors cannot be silently swallowed. Every result is either used, propagated, or explicitly handled.

<div class="callout callout-warn"><strong>panic is not error handling</strong>
<code>panic("message")</code> terminates the program immediately with a stack trace. It cannot be caught. Use it for programming errors — violated invariants, impossible states, bugs. Use result types for expected failures — bad input, missing files, network timeouts. If a user can cause it, it's not a panic.
</div>


## What's Next

Errors are values, and now you know how to create, propagate, and handle them. Next, let's define our own data types. Next up: [Structs](structs.html).
