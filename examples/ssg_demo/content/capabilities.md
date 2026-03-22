title: Capabilities
date: 2026-03-13
description: Your program only does what you allow it to do.

Most programs have unrestricted access to the outside world. They can read any file, connect to any server, access any environment variable — and you only find out what they actually do by reading every line of code.

Basalt inverts this. A program declares its capabilities in `main`'s parameter list, and the runtime provides only what is requested. If you don't ask for file system access, your program physically cannot read or write files.

## A minimal program

```basalt
fn main(stdout: Stdout) {
    stdout.println("Hello, world!")
}
```

This program can write to the terminal. It cannot read files, access environment variables, or read user input. Not because of a policy — because the runtime never gave it the objects to do those things.

## Adding capabilities

Need to read files? Add `Fs` to the parameter list:

```basalt
fn main(stdout: Stdout, fs: Fs) {
    guard let content = fs.read_file("config.txt") else {
        stdout.println("No config found")
        return
    }
    stdout.println(content)
}
```

Need command-line arguments? Add `Env`:

```basalt
fn main(stdout: Stdout, env: Env) {
    let args = env.args()
    if args.length == 0 {
        stdout.println("Usage: program <name>")
        return
    }
    stdout.println("Hello, \(args[0])!")
}
```

Every capability you use is visible in one place — the function signature.

<div class="callout callout-note"><strong>This is capability-based security, not access control</strong>
Access control systems have a central authority that grants or denies permissions at runtime. Capability-based security is structural: if you don't have the object, you can't use it. There is no way to "escalate" — no global namespace to reach into, no ambient authority to exploit.
</div>

## The Fs sandbox

The `Fs` capability is sandboxed to the directory containing the source file. Path traversal is blocked:

```basalt
fn main(stdout: Stdout, fs: Fs) {
    // This works — reading a file in the project directory
    guard let readme = fs.read_file("README.md") else {
        stdout.println("no readme")
        return
    }
    stdout.println(readme)

    // This is blocked — trying to escape the sandbox
    let result = fs.read_file("../../etc/passwd")
    match result {
        !err => stdout.println("Blocked: " + err)
        _ => {}
    }
}
```

> **Error:** path traversal blocked: ../../etc/passwd is outside the sandbox

Even with `Fs`, you can only reach files in your project directory. A downloaded script cannot read your SSH keys or browser cookies.

## Capability reference

### Stdout

| Method | Description |
|---|---|
| `stdout.println(s)` | Print with newline |
| `stdout.print(s)` | Print without newline |
| `stdout.flush()` | Flush output buffer |

### Stdin

| Method | Returns | Description |
|---|---|---|
| `stdin.read_line()` | `string` | Read one line (strips newline) |
| `stdin.read_key()` | `string` | Read single character (raw mode) |

### Fs (File System)

| Method | Returns | Description |
|---|---|---|
| `fs.read_file(path)` | `string!string` | Read file contents |
| `fs.write_file(path, data)` | `nil!string` | Write file |
| `fs.read_dir(path)` | `[string]!string` | List directory entries |
| `fs.exists(path)` | `bool` | Check if path exists |
| `fs.is_dir(path)` | `bool` | Check if path is directory |
| `fs.mkdir(path)` | `nil!string` | Create directory |
| `fs.join(a, b)` | `string` | Join path components |
| `fs.extension(path)` | `string?` | File extension (no dot) |
| `fs.stem(path)` | `string?` | Filename without extension |

### Env

| Method | Returns | Description |
|---|---|---|
| `env.args()` | `[string]` | Command-line arguments |
| `env.get(name)` | `string?` | Environment variable |

## Passing capabilities to helpers

Capabilities are regular values. Pass them as function parameters:

```basalt
fn load_config(fs: Fs) -> string!string {
    return fs.read_file("config.txt")
}

fn log(msg: string, stdout: Stdout) {
    stdout.println("[LOG] " + msg)
}

fn main(stdout: Stdout, fs: Fs) {
    log("starting up", stdout)
    guard let config = load_config(fs) else {
        log("no config found, using defaults", stdout)
        return
    }
    log("loaded config: " + (config.length as string) + " bytes", stdout)
}
```

Every function that performs I/O declares exactly which capability it needs. You can read any function's signature and know whether it touches the file system, the network, or the environment. No hidden side effects.

Without `Fs`, file access is a compile error — there's no API to call:

```basalt
fn main(stdout: Stdout) {
    let data = fs.read_file("config.txt")
    // error: undefined variable 'fs'
}
```

Add `fs: Fs` and the code compiles. Remove it and it doesn't. The type system is the security boundary.

## What's Next

The last piece of the language: [Type Conversions](conversions.html) — how Basalt handles casting between types, and why it never guesses.
