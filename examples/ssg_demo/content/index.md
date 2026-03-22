title: Learn Basalt
date: 2026-03-01
description: A friendly guide to a language that doesn't waste your time

Welcome to Basalt. This guide teaches the language from first principles, with examples you can run at every step. No prior Basalt experience needed — just a terminal and curiosity.

## Your first three minutes

Install Basalt and build the compiler:

```sh
git clone https://github.com/ForestKatsch/basalt
cd basalt
cargo build --release
```

Create a file called `hello.bas`:

```basalt
fn main(stdout: Stdout) {
    stdout.println("Hello! You're running Basalt.")
}
```

Run it:

```sh
basalt run hello.bas
```

```
Hello! You're running Basalt.
```

That's it. You just wrote and ran a Basalt program.

## The capability system

The `stdout: Stdout` parameter is how I/O works in Basalt. Programs receive capabilities as parameters — `Stdout` for printing, `Fs` for file access, `Env` for environment variables. If a capability isn't in the parameter list, the program can't use it.

You can read any function signature and know what side effects it performs. A function that takes no capability parameters is pure — it can't touch the outside world. We'll explore this fully in [Capabilities](capabilities.html).

## Type-check without running

You don't have to run your program to find mistakes. Basalt's type checker catches errors before a single line executes:

```sh
basalt check hello.bas
```

<div class="callout callout-tip"><strong>Try this</strong>
Change <code>stdout.println</code> to <code>stdout.printn</code> and run <code>basalt check hello.bas</code>. Basalt will point at the exact character, show you the source line, and suggest what you meant. Error messages are a feature, not an afterthought.
</div>

## How this guide is organized

Each chapter builds on the last, layering concepts so nothing feels like magic. Every chapter has examples you can paste into a `.bas` file and run. Start with [Types](types.html) and work your way through.
