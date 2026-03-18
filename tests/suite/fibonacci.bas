fn fib(n: i64) -> i64 {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn main(stdout: Stdout) {
    for i in 0..10 {
        stdout.println(fib(i) as string)
    }
}
