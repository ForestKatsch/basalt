fn main(stdout: Stdout) {
    // While loop
    let mut i = 0
    while i < 5 {
        i = i + 1
    }
    stdout.println(i as string)

    // For range
    let mut sum = 0
    for j in 0..10 {
        sum = sum + j
    }
    stdout.println(sum as string)

    // Loop with break
    let mut count = 0
    loop {
        count = count + 1
        if count >= 10 {
            break
        }
    }
    stdout.println(count as string)

    // Continue
    let mut evens = 0
    for k in 0..10 {
        if k % 2 != 0 {
            continue
        }
        evens = evens + 1
    }
    stdout.println(evens as string)

    // Nested loops
    let mut pairs = 0
    for x in 0..3 {
        for y in 0..3 {
            pairs = pairs + 1
        }
    }
    stdout.println(pairs as string)
}
