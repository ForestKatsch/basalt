type Color { Red, Green, Blue }

type Option {
    Some(i64)
    None
}

type Shape {
    Circle(f64)
    Rect(f64, f64)
}

fn describe(s: Shape) -> string {
    match s {
        Shape.Circle(r) => return (r as string)
        Shape.Rect(w, h) => return (w as string) + "x" + (h as string)
    }
    return "unknown"
}

fn main(stdout: Stdout) {
    // Simple enum
    let c = Color.Red
    stdout.println("enum created")

    // Enum with data
    let x = Option.Some(42)
    match x {
        Option.Some(val) => stdout.println(val as string)
        Option.None => stdout.println("none")
    }

    // Shape test
    let circle = Shape.Circle(5.0)
    let rect = Shape.Rect(3.0, 4.0)
    stdout.println(describe(circle))
    stdout.println(describe(rect))
}
