fn main(stdout: Stdout) {
    let m = {"name": "Alice", "city": "NYC"}

    // Length
    stdout.println(m.length as string)

    // Contains key
    stdout.println(m.contains_key("name") as string)
    stdout.println(m.contains_key("age") as string)

    // Keys and values
    let keys = m.keys()
    stdout.println(keys.join(", "))
}
