fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
    return f(x)
}

fn main(stdout: Stdout) {
    let double = fn(x: i64) -> i64 { return x * 2 }
    let square = fn(x: i64) -> i64 { return x * x }

    stdout.println(double(5) as string)
    stdout.println(square(5) as string)
    stdout.println(apply(double, 10) as string)
    stdout.println(apply(square, 10) as string)
}
