title: Modules
date: 2026-03-11
description: Module system and imports

Each `.bas` file is a module. Import with the `import` keyword.

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
    stdout.println(geometry.distance(a, b) as string)
}
```

All imported names are accessed with the module qualifier. There is no unqualified import. Circular imports are prohibited.

Import with alias:

```basalt
import "lib/utils" as helpers
```

Standard library modules use the `std/` prefix:

```basalt
import "std/math"

fn main(stdout: Stdout) {
    stdout.println(math.sqrt(2.0) as string)
}
```
