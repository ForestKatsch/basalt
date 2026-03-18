fn main(stdout: Stdout) {
    // Int to string
    stdout.println(42 as string)

    // Float to string
    stdout.println(3.14 as string)

    // Bool to string
    stdout.println(true as string)
    stdout.println(false as string)

    // Int to float
    let x = 42 as f64
    stdout.println((x + 0.5) as string)

    // Float to int (truncation toward zero)
    stdout.println((3.9 as i64) as string)
    stdout.println((-3.9 as i64) as string)
}
