title: Modules
date: 2026-03-11
description: Each file is a module. All access is qualified. No ambiguity.

In Basalt, every `.bas` file is a module. There is no module declaration syntax — the filename is the module name. You import a module, then access its contents through the module name.

## Importing and using modules

```basalt
// geometry.bas
type Point {
    x: f64
    y: f64
}

fn distance(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x
    let dy = a.y - b.y
    return math.sqrt(dx * dx + dy * dy)
}
```

```basalt
// main.bas
import "geometry"

fn main(stdout: Stdout) {
    let a = geometry.Point { x: 0.0, y: 0.0 }
    let b = geometry.Point { x: 3.0, y: 4.0 }
    stdout.println(geometry.distance(a, b) as string)  // Output: 5
}
```

All access is qualified: `geometry.Point`, `geometry.distance`. There is no unqualified import — you always know where a name comes from by reading the call site.

<div class="callout callout-note"><strong>Why qualified access?</strong>
When two modules export the same name, qualified access makes collisions impossible. You never have to guess which <code>parse</code> or <code>Error</code> you are looking at. The tradeoff is verbosity — but the code tells the truth about its dependencies at every use site.
</div>

## Import aliases

When a module path is long, give it a shorter name:

```basalt
import "lib/validators" as val

fn main(stdout: Stdout) {
    let ok = val.is_email("user@example.com")
    stdout.println(ok as string)
}
```

The alias replaces the module name at all use sites. The original long name is not accessible.

## Standard library imports

Standard library modules live under the `std/` prefix:

```basalt
import "std/math"

fn main(stdout: Stdout) {
    stdout.println(math.sqrt(2.0) as string)  // Output: 1.4142135623730951
    stdout.println(math.pi as string)         // Output: 3.141592653589793
}
```

Built-in types like `string`, `i64`, and array methods are available without any import. You only need `import` for the `math` module.

## Subdirectories

Modules can live in subdirectories. The import path reflects the file path:

```basalt
import "models/user"
import "handlers/auth"

fn main(stdout: Stdout, fs: Fs) {
    let u = user.load(fs)
    auth.verify(u, stdout)
}
```

## No circular imports

Basalt forbids circular imports. If module A imports module B, then module B cannot import module A — directly or through any chain of intermediate imports.

> **Error:** circular import detected: main.bas → auth.bas → main.bas

This constraint keeps your dependency graph a tree, which makes builds fast and dependencies easy to reason about. If two modules need to share types, extract the shared types into a third module that both import.

## What's Next

Now that you can organize code across files, explore the [Standard Library](stdlib.html) — the built-in methods on strings, arrays, maps, and the math module.
