type Point {
    x: f64
    y: f64
}

fn main(stdout: Stdout) {
    let p = Point { x: 3.0, y: 4.0 }
    stdout.println(p.x as string)
    stdout.println(p.y as string)

    // Reference semantics
    let q = p
    q.x = 10.0
    stdout.println(p.x as string)

    // Clone for independent copy
    let r = p.clone()
    r.x = 99.0
    stdout.println(p.x as string)
    stdout.println(r.x as string)
}
