# Embedding Basalt

This document describes how to embed the Basalt compiler and VM in a Rust
application. For the language specification, see [SPEC.md](SPEC.md).

## Compilation

```rust
use basalt_core;
use std::path::Path;

// Compile from source string
let program = basalt_core::compile(source)?;

// Compile from file (resolves imports relative to file's directory)
let program = basalt_core::compile_file(Path::new("program.bas"))?;

// Compile with rich error info (source + filename for diagnostics)
let result = basalt_core::compile_file_rich(Path::new("program.bas"))?;
let program = result.program;
```

## VM Interaction

```rust
use basalt_vm::VM;
use std::path::PathBuf;

let mut vm = VM::new(program);

// Configure capabilities before running
vm.set_fs_root(PathBuf::from("/sandboxed/directory"));
vm.set_env_args(vec!["arg1".into(), "arg2".into()]);
vm.set_stdin(vec!["line1".into(), "line2".into()]);

// Execute
let result = vm.run()?;

// Captured stdout output (available even when not connected to a terminal)
for line in &vm.captured_output {
    println!("{}", line);
}
```

## Capabilities

The VM constructs capability objects for `main`'s parameters based on their
types. The host controls what capabilities are available:

| Parameter type | Capability | Host configuration |
|---|---|---|
| `Stdout` | Terminal output | Always available |
| `Stdin` | Terminal input | `vm.set_stdin(lines)` for testing |
| `Fs` | Sandboxed file system | `vm.set_fs_root(path)` — all paths resolved relative to root, cannot escape |
| `Env` | Program arguments, environment variables | `vm.set_env_args(args)` |
| `Highlight` | Syntax highlighting | Always available |

A program that doesn't request a capability cannot use it. There is no way
for Basalt code to forge or escalate capabilities.

## Value Types

The `Value` type represents a Basalt runtime value. The host must use the
correct constructor — the VM interprets values according to the compiler's
type information, not runtime tags.

```rust
use basalt_vm::Value;

Value::int(42)
Value::float(3.14)
Value::bool(true)
Value::string("hello".to_string())
Value::array(vec![Value::int(1), Value::int(2)])
Value::Nil
```

## Runtime Limits

| Limit | Default | Description |
|---|---|---|
| Max call depth | 256 | Prevents stack overflow from deep recursion |
| Max instructions | 100M | Prevents infinite loops (execution fuel) |
| Max string repeat | 16 MiB | Prevents OOM from `string.repeat()` |

## Host Objects

The embedding host can inject custom capability objects by implementing
the `HostObject` trait (planned — not yet implemented):

```rust
struct GameApi;
impl HostObject for GameApi {
    fn call_method(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "get_time" => Ok(Value::float(0.0)),
            _ => Err(format!("unknown method: {name}"))
        }
    }
    fn type_name(&self) -> &str { "GameApi" }
}
vm.set_global("game", Value::host_object(GameApi));
```

**Note:** The `HostObject` trait and `Value::host_object` constructor are
specified but not yet implemented. Custom host capabilities require changes
to the VM source. See [DEVELOPMENT.md](DEVELOPMENT.md) for the current
capability architecture.

## Error Handling

Compilation errors are returned as `CompileError` (or `String` via the
legacy `compile`/`compile_file` API). Runtime errors are returned as
`String` from `vm.run()`.

For rich diagnostics with source context:

```rust
use basalt_core::compile_file_rich;

match compile_file_rich(Path::new("program.bas")) {
    Ok(result) => {
        // result.program, result.source, result.filename
    }
    Err((error, source, filename)) => {
        // Render with source context, line numbers, carets
        eprintln!("{}", error.render(&source, &filename));
    }
}
```
