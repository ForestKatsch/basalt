title: Learn Basalt
date: 2026-03-01
description: A friendly guide to a language that doesn't waste your time

Welcome to Basalt. This guide teaches the language from first principles, with examples you can run at every step. No prior Basalt experience needed — just a terminal and curiosity.

## Your first three minutes

Install Basalt and build the compiler:

```sh
git clone https://github.com/example/basalt
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

Notice the `stdout: Stdout` parameter on `main`. That's not decoration — it's the **capability system**. Your program only gets access to the resources it asks for. No `Stdout` parameter, no printing. No `Fs` parameter, no file system access. No `Net` parameter, no network.

This means you can read any Basalt function signature and know exactly what side effects it can perform. A function that takes no capability parameters is pure — it can't touch the outside world. We'll explore this fully in [Capabilities](capabilities.html).

## Type-check without running

You don't have to run your program to find mistakes. Basalt's type checker catches errors before a single line executes:

```sh
basalt check hello.bas
```

<div class="callout callout-tip"><strong>Try this</strong>
Change <code>stdout.println</code> to <code>stdout.printn</code> and run <code>basalt check hello.bas</code>. Basalt will point at the exact character, show you the source line, and suggest what you meant. Error messages are a feature, not an afterthought.
</div>

## How this guide is organized

Each chapter builds on the last. You'll learn the type system, then variables, then functions — layering concepts so nothing feels like magic. Every chapter has examples you can paste into a `.bas` file and run.

1. [Types](types.html) — every value has a type, every type tells the truth
2. [Variables](variables.html) — immutable by default, mutable when you need it
3. [Functions](functions.html) — explicit parameters, return types, and returns
4. [Control Flow](control-flow.html) — conditions, loops, and guards
5. [Pattern Matching](pattern-matching.html) — the compiler checks every case
6. [Error Handling](error-handling.html) — errors are values, not surprises
7. [Structs](structs.html) — define your own data types
8. [Enums](enums.html) — variants with associated data
9. [Closures](closures.html) — functions that capture their environment
10. [Modules](modules.html) — organize code across files
11. [Standard Library](stdlib.html) — what's built in
12. [Capabilities](capabilities.html) — controlled access to the outside world
13. [Type Conversions](conversions.html) — safe, explicit, no surprises
