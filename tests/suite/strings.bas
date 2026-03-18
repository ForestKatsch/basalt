fn main(stdout: Stdout) {
    let s = "Hello, World!"

    stdout.println(s.length as string)
    stdout.println(s.upper())
    stdout.println(s.lower())
    stdout.println(s.contains("World") as string)
    stdout.println(s.starts_with("Hello") as string)
    stdout.println(s.ends_with("!") as string)
    stdout.println(s.replace("World", "Basalt"))
    stdout.println("  spaces  ".trim())
    stdout.println("abc".repeat(3))
    stdout.println(s.substring(0, 5))
    stdout.println(s.char_at(0))
    stdout.println(s.char_at(-1))

    // Split
    let parts = "a,b,c".split(",")
    stdout.println(parts.length as string)
    stdout.println(parts.join(" | "))

    // Concatenation
    stdout.println("Hello" + ", " + "World!")
}
