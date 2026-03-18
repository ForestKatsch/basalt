fn main(stdout: Stdout) {
    // Integer arithmetic
    let a = 10
    let b = 3
    stdout.println((a + b) as string)
    stdout.println((a - b) as string)
    stdout.println((a * b) as string)
    stdout.println((a / b) as string)
    stdout.println((a % b) as string)

    // Float arithmetic
    let x = 10.0
    let y = 3.0
    stdout.println((x + y) as string)
    stdout.println((x - y) as string)
    stdout.println((x * y) as string)
    stdout.println((x / y) as string)

    // Power
    stdout.println((2 ** 10) as string)

    // Unary
    stdout.println((-42) as string)
    stdout.println((!true) as string)
}
