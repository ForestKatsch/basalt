fn divide(a: f64, b: f64) -> f64!string {
    if b == 0.0 {
        return !("division by zero")
    }
    return a / b
}

fn main(stdout: Stdout) {
    // Match on result
    match divide(10.0, 2.0) {
        !err => stdout.println("Error: " + err)
        val => stdout.println("OK: " + (val as string))
    }
    match divide(10.0, 0.0) {
        !err => stdout.println("Error: " + err)
        val => stdout.println("OK: " + (val as string))
    }
}
