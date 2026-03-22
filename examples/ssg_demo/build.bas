// Static Site Generator in Basalt
// Reads .md files from content/, applies templates, writes .html to output/

// --- Markdown to HTML (simplified) ---

fn md_to_html(md: string) -> string {
    let lines = md.split("\n")
    let mut html = ""
    let mut in_list = false

    for line in lines {
        // Skip frontmatter lines (key: value at top)
        if line.contains(": ") && !line.starts_with("#") && !line.starts_with("-") && !line.starts_with(" ") {
            if !line.starts_with("<") {
                continue
            }
        }

        // Blank line
        if line.trim().length == 0 {
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            continue
        }

        // Headers
        if line.starts_with("## ") {
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            html = html + "<h2>" + inline_md(line.slice(3, line.length)) + "</h2>\n"
            continue
        }
        if line.starts_with("# ") {
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            html = html + "<h1>" + inline_md(line.slice(2, line.length)) + "</h1>\n"
            continue
        }

        // List items
        if line.starts_with("- ") {
            if !in_list {
                html = html + "<ul>\n"
                in_list = true
            }
            html = html + "<li>" + inline_md(line.slice(2, line.length)) + "</li>\n"
            continue
        }

        // Paragraph
        if in_list {
            html = html + "</ul>\n"
            in_list = false
        }
        html = html + "<p>" + inline_md(line) + "</p>\n"
    }

    if in_list {
        html = html + "</ul>\n"
    }

    return html
}

// Process inline markdown: **bold** and `code`
fn inline_md(text: string) -> string {
    let mut result = ""
    let chars = text.chars()
    let mut i = 0

    while i < chars.length {
        // Bold: **text**
        if i + 1 < chars.length && chars[i] == "*" && chars[i + 1] == "*" {
            let end = text.index_of("**")
            if end >= 0 {
                // Find closing **
                let rest = text.slice(i + 2, text.length)
                let close = rest.index_of("**")
                if close >= 0 {
                    let bold_text = rest.slice(0, close)
                    result = result + "<strong>" + bold_text + "</strong>"
                    i = i + 4 + close
                    continue
                }
            }
        }

        // Code: `text`
        if chars[i] == "`" {
            let rest = text.slice(i + 1, text.length)
            let close = rest.index_of("`")
            if close >= 0 {
                let code_text = rest.slice(0, close)
                result = result + "<code>" + code_text + "</code>"
                i = i + 2 + close
                continue
            }
        }

        result = result + chars[i]
        i = i + 1
    }

    return result
}

// --- Frontmatter parsing ---

fn parse_frontmatter(content: string) -> [string: string] {
    let mut meta: [string: string] = {}
    let lines = content.split("\n")
    for line in lines {
        if line.trim().length == 0 {
            continue
        }
        if line.starts_with("#") {
            break
        }
        let colon = line.index_of(": ")
        if colon >= 0 {
            let key = line.slice(0, colon)
            let value = line.slice(colon + 2, line.length)
            meta[key] = value
        }
    }
    return meta
}

// --- Template engine ---

fn apply_template(template: string, vars: [string: string]) -> string {
    let mut result = template
    let keys = vars.keys()
    for key in keys {
        let placeholder = "{{" + key + "}}"
        let value = vars[key]
        result = result.replace(placeholder, value)
    }
    return result
}

// --- Main build pipeline ---

fn main(stdout: Stdout, fs: Fs) {
    stdout.println("=== Basalt Static Site Generator ===")
    stdout.println("")

    // Read template
    let tmpl_result = fs.read_file("templates/page.html")
    guard let template = tmpl_result else {
        stdout.println("Error reading template")
        return
    }

    // Read content files
    let files_result = fs.read_dir("content")
    guard let files = files_result else {
        stdout.println("Error reading content/")
        return
    }

    // Filter to .md files
    let md_files = files.filter(fn(f: string) -> bool {
        let ext = fs.extension(f)
        if ext is nil { return false }
        return (ext as string) == "md"
    })

    stdout.println("Found " + md_files.length as string + " content files")

    // Build navigation
    let nav_links = md_files.map(fn(f: string) -> string {
        let name = fs.stem(f)
        if name is nil { return "" }
        let stem = name as string
        return "<a href=\"" + stem + ".html\">" + stem + "</a>"
    })
    let nav_html = nav_links.join(" ")

    // Process each file
    for file in md_files {
        let stem_opt = fs.stem(file)
        if stem_opt is nil { continue }
        let stem = stem_opt as string

        // Read markdown
        let read_result = fs.read_file(fs.join("content", file))
        guard let content = read_result else {
            stdout.println("  Error reading " + file)
            continue
        }

        // Parse frontmatter
        let meta = parse_frontmatter(content)
        let title = if meta.contains_key("title") { meta["title"] } else { stem }
        let date = if meta.contains_key("date") { meta["date"] } else { "" }

        // Convert markdown to HTML
        let body_html = md_to_html(content)

        // Apply template
        let mut vars: [string: string] = {}
        vars["title"] = title
        vars["date"] = date
        vars["body"] = body_html
        vars["nav"] = nav_html

        let page_html = apply_template(template, vars)

        // Write output
        let out_path = fs.join("output", stem + ".html")
        let write_result = fs.write_file(out_path, page_html)
        match write_result {
            !err => stdout.println("  Error writing " + out_path + ": " + err)
            _ => stdout.println("  Generated " + out_path)
        }
    }

    stdout.println("")
    stdout.println("Build complete!")
}
