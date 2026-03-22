title: Learn Basalt
date: 2026-03-01
description: A friendly guide to a language that doesn't waste your time

This guide walks you through everything Basalt can do, with examples you can run at every step. No prior experience with Basalt is needed — just curiosity.

## Your first three minutes

Install and build:

```
cargo build --release
```

Write this to a file called `hello.bas`:

```
fn main(stdout: Stdout) {
    stdout.println("Hello! You're running Basalt.")
}
```

Run it:

```
basalt run hello.bas
```

That's it. You just wrote and ran a Basalt program. The `stdout: Stdout` part is how Basalt handles I/O — your program only gets the capabilities you ask for. No Stdout parameter, no printing. We'll explain why this is a great idea in the [Capabilities](capabilities.html) chapter.

You can also type-check without running:

```
basalt check hello.bas
```

If there's an error, Basalt will point at the exact line and character, show you the source, and often suggest what you meant to write. Try introducing a typo and see what happens.
