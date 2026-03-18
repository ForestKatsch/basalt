fn check_positive(x: i64, stdout: Stdout) {
    guard x > 0 else {
        stdout.println("not positive")
        return
    }
    stdout.println("positive: " + (x as string))
}

fn main(stdout: Stdout) {
    check_positive(5, stdout)
    check_positive(-1, stdout)
    check_positive(0, stdout)
}
