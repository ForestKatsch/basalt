title: Error Handling
date: 2026-03-07
description: Result types, error propagation, and recovery

Basalt has no exceptions. Errors are values, carried by result types `T!E`.

### Creating and returning errors

```basalt
fn parse_age(s: string) -> i64!string {
    let n = s as? i64
    if n is nil {
        return !("invalid number: " + s)
    }
    let age = n as i64
    if age < 0 {
        return !("age cannot be negative")
    }
    return age
}
```

### Propagating with ?

The `?` operator propagates errors automatically. If the result is an error, it returns from the enclosing function. If successful, it unwraps the value.

```basalt
fn load_and_parse(fs: Fs) -> i64!string {
    let content = fs.read_file("age.txt")?  // propagates read error
    let age = parse_age(content.trim())?     // propagates parse error
    return age
}
```

The enclosing function must return a compatible result type.

### guard let for unwrapping

When you want to unwrap and diverge on failure without `match`:

```basalt
fn process(fs: Fs, stdout: Stdout) {
    guard let data = fs.read_file("input.txt") else {
        stdout.println("Cannot read input")
        return
    }
    // data is `string` here
    stdout.println("Read " + (data.length as string) + " chars")
}
```

### panic for unrecoverable errors

```basalt
fn main(stdout: Stdout) {
    panic("something went terribly wrong")
    // Terminates with message and stack trace. Cannot be caught.
}
```

Use `panic` for programming errors. Use result types for expected failures.
