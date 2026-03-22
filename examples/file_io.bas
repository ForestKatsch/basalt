// Demonstrates file I/O with the Fs capability.
// Fs is sandboxed to the directory containing this file.

fn main(stdout: Stdout, fs: Fs) {
    // Write a file
    let write_result = fs.write_file("output.txt", "Hello from Basalt!\nLine 2\nLine 3")
    match write_result {
        !err => {
            stdout.println("Write failed: " + err)
            return
        }
        _ => stdout.println("Wrote output.txt")
    }

    // Check it exists
    stdout.println("output.txt exists: " + fs.exists("output.txt") as string)

    // Read it back
    let read_result = fs.read_file("output.txt")
    match read_result {
        !err => stdout.println("Read failed: " + err)
        content => {
            stdout.println("Read " + content.length as string + " chars:")
            let lines = content.split("\n")
            for line in lines {
                stdout.println("  > " + line)
            }
        }
    }

    // List directory
    let dir_result = fs.read_dir(".")
    match dir_result {
        !err => stdout.println("Dir failed: " + err)
        files => {
            stdout.println("Files in directory:")
            for file in files {
                stdout.println("  " + file)
            }
        }
    }

    // Path helpers
    let p = fs.join("subdir", "page.html")
    stdout.println("Joined path: " + p)

    let ext = fs.extension("photo.png")
    if ext is nil {
        stdout.println("No extension")
    } else {
        stdout.println("Extension: " + ext as string)
    }

    // Clean up
    let _ = fs.write_file("output.txt", "")
}
