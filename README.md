# Basalt

A statically-typed, bytecode-compiled programming language built in Rust.

Basalt emphasizes **strict typing**, **explicit semantics**, and **capability-based I/O**. There are no implicit conversions, no exceptions, no null, and no hidden control flow.

## Quick Start

```bash
cargo build --release
./target/release/basalt run tests/suite/hello.bas
```

## Example

```
fn fib(n: i64) -> i64 {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn main(stdout: Stdout) {
    stdout.println(fib(30) as string)
}
```

## Design Principles

- **Strict static typing** -- every value has a type known at compile time. `1 + 2.0` is a compile error.
- **Explicit over implicit** -- no hidden control flow, no magic methods, no default arguments, no operator overloading.
- **Errors are values** -- no exceptions. Fallible operations return `T!E` result types. `?` propagates errors.
- **Capability-based I/O** -- programs cannot perform I/O unless the host grants it. `main(stdout: Stdout)` receives capabilities as parameters.
- **No null** -- use `T?` (optional) when a value may be absent, or `nil` as an explicit unit value.

## Type System

| Category | Types |
|----------|-------|
| Integers | `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64` |
| Float | `f64` |
| Other primitives | `bool`, `string`, `nil` |
| Collections | `[T]` arrays, `[K: V]` maps, `(T1, T2)` tuples |
| Composite | `type` structs, enums with data-carrying variants |
| Algebraic | `A \| B` union types, `T?` optionals, `T!E` results |

## Architecture

Basalt compiles source to bytecode and executes it in a register-based VM:

```
Source (.bas) -> Lexer -> Parser -> Type Checker -> Compiler -> VM
```

The project is structured as three crates:

| Crate | Role |
|-------|------|
| `basalt-core` | Frontend: lexer, parser, type checker, bytecode compiler |
| `basalt-vm` | Backend: register-based bytecode VM |
| `basalt` | CLI: `basalt run <file.bas>` |

## Running Tests

```bash
cargo test
```

## Language Specification

See [SPEC.md](SPEC.md) for the full language specification including syntax, semantics, the type system, standard library, and embedding API.

## License

MIT
