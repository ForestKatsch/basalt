fn main(stdout: Stdout) {
    let arr = [10, 20, 30, 40, 50]

    // Length
    stdout.println(arr.length as string)

    // Indexing
    stdout.println(arr[0] as string)
    stdout.println(arr[-1] as string)

    // Push and pop
    arr.push(60)
    stdout.println(arr.length as string)
    let last = arr.pop()
    stdout.println(last as string)

    // Reference semantics
    let alias = arr
    alias.push(99)
    stdout.println(arr.length as string)

    // Clone
    let copy = arr.clone()
    copy.push(100)
    stdout.println(arr.length as string)
    stdout.println(copy.length as string)

    // Join
    let nums = [1, 2, 3]
    stdout.println(nums.join(", "))

    // Contains
    stdout.println(arr.contains(10) as string)
    stdout.println(arr.contains(999) as string)

    // For loop
    let sum = [1, 2, 3, 4, 5]
    let mut total = 0
    for val in sum {
        total = total + val
    }
    stdout.println(total as string)
}
