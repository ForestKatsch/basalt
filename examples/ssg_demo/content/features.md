title: Language Features
date: 2026-03-21
description: A tour of Basalt's features

# Language Features

Basalt includes everything needed to build real programs.

## Pattern Matching

Exhaustive `match` expressions ensure every case is handled:

```
match color {
    Color.Red => "red"
    Color.Green => "green"
    Color.Blue => "blue"
}
```

## Error Handling

Errors are values, not exceptions. The `?` operator propagates errors:

```
fn read_config(fs: Fs) -> Config!string {
    let data = fs.read_file("config.json")?
    return parse_config(data)
}
```

## Closures

First-class functions with closure capture:

```
let nums = [1, 2, 3, 4, 5]
let evens = nums.filter(fn(x: i64) -> bool {
    return x % 2 == 0
})
```

---

*Explore the [full documentation](https://github.com/example/basalt) for more.*
