fn classify(n: i64) -> string {
    match n {
        0 => return "zero"
        1 => return "one"
        _ => return "other"
    }
    return "unreachable"
}

fn main(stdout: Stdout) {
    stdout.println(classify(0))
    stdout.println(classify(1))
    stdout.println(classify(42))

    // Match with bool
    let x = true
    match x {
        true => stdout.println("yes")
        false => stdout.println("no")
    }

    // Match with string
    let cmd = "quit"
    match cmd {
        "start" => stdout.println("starting")
        "stop" => stdout.println("stopping")
        "quit" => stdout.println("quitting")
        _ => stdout.println("unknown")
    }
}
