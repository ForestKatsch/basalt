title: Capabilities
date: 2026-03-13
description: Capability-based I/O system

IO in Basalt works through capability objects passed to `main`. A program can only perform IO that it explicitly requests.

### Stdout

```basalt
fn main(stdout: Stdout) {
    stdout.println("line with newline")
    stdout.print("no newline")
    stdout.flush()
}
```

### Stdin

```basalt
fn main(stdout: Stdout, stdin: Stdin) {
    stdout.print("What is your name? ")
    stdout.flush()
    let name = stdin.read_line()
    stdout.println("Hello, \(name)!")
}
```

| Method | Description |
|---|---|
| `stdin.read_line()` | Read one line (strips newline) |
| `stdin.read_key()` | Read single character (raw mode) |

### Fs (File System)

The `Fs` capability is sandboxed to the directory containing the source file.

```basalt
fn main(stdout: Stdout, fs: Fs) {
    // Write a file
    let write_result = fs.write_file("output.txt", "Hello from Basalt!")
    match write_result {
        !err => stdout.println("Write failed: " + err)
        _ => stdout.println("Wrote output.txt")
    }

    // Read a file
    guard let content = fs.read_file("output.txt") else {
        stdout.println("Read failed")
        return
    }
    stdout.println(content)

    // Check existence
    stdout.println(fs.exists("output.txt") as string)  // Output: true

    // List directory
    guard let files = fs.read_dir(".") else { return }
    for file in files {
        stdout.println(file)
    }

    // Path utilities
    let p = fs.join("subdir", "page.html")   // "subdir/page.html"
    let ext = fs.extension("photo.png")       // "png" (returns string?)
    let name = fs.stem("photo.png")           // "photo" (returns string?)
}
```

| Method | Returns | Description |
|---|---|---|
| `fs.read_file(path)` | `string!string` | Read file contents |
| `fs.write_file(path, data)` | `nil!string` | Write file |
| `fs.read_dir(path)` | `[string]!string` | List directory entries |
| `fs.exists(path)` | `bool` | Check if path exists |
| `fs.is_dir(path)` | `bool` | Check if path is a directory |
| `fs.mkdir(path)` | `nil!string` | Create directory |
| `fs.join(a, b)` | `string` | Join path components |
| `fs.extension(path)` | `string?` | File extension without dot |
| `fs.stem(path)` | `string?` | Filename without extension |

### Env

```basalt
fn main(stdout: Stdout, env: Env) {
    let args = env.args()
    for arg in args {
        stdout.println(arg)
    }

    let home = env.get("HOME")
    if home is string {
        stdout.println("Home: " + home)
    }
}
```

| Method | Returns | Description |
|---|---|---|
| `env.args()` | `[string]` | Command-line arguments |
| `env.get(name)` | `string?` | Environment variable value |

### How capabilities work

Capabilities are passed as parameters to `main`. You only get what you ask for:

```basalt
// This program can print but cannot read files
fn main(stdout: Stdout) {
    stdout.println("I have no file access")
}

// This program can read/write files and print
fn main(stdout: Stdout, fs: Fs) {
    guard let data = fs.read_file("input.txt") else { return }
    stdout.println(data)
}
```

Pass capabilities to helper functions as regular parameters:

```basalt
fn log(msg: string, stdout: Stdout) {
    stdout.println("[LOG] " + msg)
}

fn main(stdout: Stdout) {
    log("starting up", stdout)
}
```
