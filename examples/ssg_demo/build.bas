// Static Site Generator in Basalt
// Reads .md files from content/, converts markdown to HTML, applies templates,
// generates an index page, and writes everything to output/.

// --- HTML escaping ---
// Replaces &, <, > with their HTML entities.
// Must be called on text content BEFORE wrapping in HTML tags,
// but NOT on already-generated HTML or code block contents.
fn escape_html(text: string) -> string {
    // Order matters: & first, otherwise we'd double-escape
    let mut s = text.replace("&", "&amp;")
    s = s.replace("<", "&lt;")
    s = s.replace(">", "&gt;")
    return s
}

// --- Inline markdown ---
// Processes: **bold**, *italic*, `code`, [text](url)
// Expects text that has NOT been HTML-escaped yet (escapes non-special text itself).
fn inline_md(text: string) -> string {
    let chars = text.chars()
    let len = chars.length
    let mut result = ""
    let mut i = 0

    while i < len {
        // Bold: **text**
        if i + 1 < len && chars[i] == "*" && chars[i + 1] == "*" {
            let rest = text.slice(i + 2, len)
            let close = rest.index_of("**")
            if close >= 0 {
                let inner = rest.slice(0, close)
                result = result + "<strong>" + inline_md(inner) + "</strong>"
                i = i + 4 + close
                continue
            }
        }

        // Italic: *text* (single asterisk, but not **)
        if chars[i] == "*" && (i + 1 >= len || chars[i + 1] != "*") {
            let rest = text.slice(i + 1, len)
            let close = rest.index_of("*")
            if close >= 0 {
                let inner = rest.slice(0, close)
                result = result + "<em>" + inline_md(inner) + "</em>"
                i = i + 2 + close
                continue
            }
        }

        // Inline code: `text`
        if chars[i] == "`" {
            let rest = text.slice(i + 1, len)
            let close = rest.index_of("`")
            if close >= 0 {
                // Code content is escaped but not processed for markdown
                let code_text = rest.slice(0, close)
                result = result + "<code>" + escape_html(code_text) + "</code>"
                i = i + 2 + close
                continue
            }
        }

        // Link: [text](url)
        if chars[i] == "[" {
            let rest = text.slice(i + 1, len)
            let close_bracket = rest.index_of("]")
            if close_bracket >= 0 {
                // Check for ( immediately after ]
                let after_bracket = i + 1 + close_bracket + 1
                if after_bracket < len && chars[after_bracket] == "(" {
                    let url_start = after_bracket + 1
                    let url_rest = text.slice(url_start, len)
                    let close_paren = url_rest.index_of(")")
                    if close_paren >= 0 {
                        let link_text = rest.slice(0, close_bracket)
                        let url = url_rest.slice(0, close_paren)
                        result = result + "<a href=\"" + url + "\">" + escape_html(link_text) + "</a>"
                        i = url_start + close_paren + 1
                        continue
                    }
                }
            }
        }

        // Regular character — escape individually
        let ch = chars[i]
        if ch == "&" {
            result = result + "&amp;"
        } else {
            if ch == "<" {
                result = result + "&lt;"
            } else {
                if ch == ">" {
                    result = result + "&gt;"
                } else {
                    result = result + ch
                }
            }
        }
        i = i + 1
    }

    return result
}

// --- Table row splitting helper ---
fn split_table_row(line: string) -> [string] {
    let parts = line.split("|")
    let mut cells: [string] = []
    let mut idx = 1
    while idx < parts.length - 1 {
        cells.push(parts[idx].trim())
        idx = idx + 1
    }
    return cells
}

// --- Block-level markdown to HTML ---
fn md_to_html(md: string, highlight: Highlight) -> string {
    let lines = md.split("\n")
    let mut html = ""
    let mut in_list = false
    let mut in_table = false
    let mut in_code_block = false
    let mut code_content = ""
    let mut code_lang = ""
    let mut in_blockquote = false
    let mut para_lines: [string] = []
    let mut i = 0

    while i < lines.length {
        let line = lines[i]
        i = i + 1

        // Code block toggle: ```
        if line.trim().starts_with("```") {
            if in_code_block {
                // Close code block — content was accumulated raw
                html = html + "<pre><code>" + highlight.code(code_content, code_lang) + "</code></pre>\n"
                code_content = ""
                code_lang = ""
                in_code_block = false
            } else {
                // Flush any open structures before opening code block
                if in_table {
                    html = html + "</tbody></table>\n"
                    in_table = false
                }
                if para_lines.length > 0 {
                    html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                    para_lines = []
                }
                if in_list {
                    html = html + "</ul>\n"
                    in_list = false
                }
                if in_blockquote {
                    html = html + "</blockquote>\n"
                    in_blockquote = false
                }
                // Extract language tag from ```lang
                let trimmed = line.trim()
                if trimmed.length > 3 {
                    code_lang = trimmed.slice(3, trimmed.length)
                } else {
                    code_lang = ""
                }
                in_code_block = true
            }
            continue
        }

        // Inside code block: accumulate raw lines, no processing
        if in_code_block {
            if code_content.length > 0 {
                code_content = code_content + "\n" + line
            } else {
                code_content = line
            }
            continue
        }

        // Blank line: flush paragraph, close list/blockquote
        if line.trim().length == 0 {
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            if in_blockquote {
                html = html + "</blockquote>\n"
                in_blockquote = false
            }
            continue
        }

        // Horizontal rule: --- on its own
        if line.trim() == "---" {
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            html = html + "<hr>\n"
            continue
        }

        // Headers: ### before ## before #
        if line.starts_with("### ") {
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            html = html + "<h3>" + inline_md(line.slice(4, line.length)) + "</h3>\n"
            continue
        }
        if line.starts_with("## ") {
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            html = html + "<h2>" + inline_md(line.slice(3, line.length)) + "</h2>\n"
            continue
        }
        if line.starts_with("# ") {
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            html = html + "<h1>" + inline_md(line.slice(2, line.length)) + "</h1>\n"
            continue
        }

        // Unordered list: - item
        if line.starts_with("- ") {
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if in_blockquote {
                html = html + "</blockquote>\n"
                in_blockquote = false
            }
            if !in_list {
                html = html + "<ul>\n"
                in_list = true
            }
            html = html + "<li>" + inline_md(line.slice(2, line.length)) + "</li>\n"
            continue
        }

        // Blockquote: > text
        if line.starts_with("> ") {
            if para_lines.length > 0 {
                html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                para_lines = []
            }
            if in_table {
                html = html + "</tbody></table>\n"
                in_table = false
            }
            if in_list {
                html = html + "</ul>\n"
                in_list = false
            }
            if !in_blockquote {
                html = html + "<blockquote>\n"
                in_blockquote = true
            }
            html = html + "<p>" + inline_md(line.slice(2, line.length)) + "</p>\n"
            continue
        }

        // Table: line starts with |
        if line.starts_with("|") {
            if !in_table {
                // Start table — flush open structures
                if para_lines.length > 0 {
                    html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
                    para_lines = []
                }
                if in_list {
                    html = html + "</ul>\n"
                    in_list = false
                }
                if in_blockquote {
                    html = html + "</blockquote>\n"
                    in_blockquote = false
                }
                in_table = true
                let cells = split_table_row(line)
                html = html + "<table>\n<thead><tr>"
                for cell in cells {
                    html = html + "<th>" + inline_md(cell) + "</th>"
                }
                html = html + "</tr></thead>\n<tbody>\n"
                continue
            }
            // Separator row (|---|---|): skip
            if line.contains("---") {
                continue
            }
            // Data row
            let cells = split_table_row(line)
            html = html + "<tr>"
            for cell in cells {
                html = html + "<td>" + inline_md(cell) + "</td>"
            }
            html = html + "</tr>\n"
            continue
        }
        // Close table if we were in one
        if in_table {
            html = html + "</tbody></table>\n"
            in_table = false
        }

        // Regular text line: accumulate for paragraph
        if in_list {
            html = html + "</ul>\n"
            in_list = false
        }
        if in_blockquote {
            html = html + "</blockquote>\n"
            in_blockquote = false
        }
        para_lines.push(line)
    }

    // Flush any remaining open structures
    if para_lines.length > 0 {
        html = html + "<p>" + inline_md(para_lines.join(" ")) + "</p>\n"
    }
    if in_table {
        html = html + "</tbody></table>\n"
    }
    if in_list {
        html = html + "</ul>\n"
    }
    if in_blockquote {
        html = html + "</blockquote>\n"
    }
    if in_code_block {
        // Unclosed code block — emit what we have
        html = html + "<pre><code>" + highlight.code(code_content, code_lang) + "</code></pre>\n"
    }

    return html
}

// --- Frontmatter parsing ---
// Reads key: value pairs from the top of the file until a blank line or # header.
fn parse_frontmatter(content: string) -> [string: string] {
    let mut meta: [string: string] = {}
    let lines = content.split("\n")
    for line in lines {
        if line.trim().length == 0 {
            break
        }
        if line.starts_with("#") {
            break
        }
        let colon = line.index_of(": ")
        if colon >= 0 {
            let key = line.slice(0, colon).trim()
            let value = line.slice(colon + 2, line.length).trim()
            meta[key] = value
        }
    }
    return meta
}

// --- Strip frontmatter and leading title from content ---
// Returns everything after the frontmatter block (after the first blank line).
// Also removes the first H1 header if it matches the title (since the template renders it).
fn strip_frontmatter(content: string) -> string {
    let lines = content.split("\n")
    let mut started = false
    let mut skipped_title = false
    let mut result_lines: [string] = []
    for line in lines {
        if started {
            // Skip the first H1 (# Title) since template renders title separately
            if !skipped_title && line.starts_with("# ") && !line.starts_with("## ") {
                skipped_title = true
                continue
            }
            result_lines.push(line)
        } else {
            // Frontmatter ends at blank line or at first # header
            if line.trim().length == 0 {
                started = true
            }
            if line.starts_with("#") {
                started = true
                // Don't push H1 title (skip it), but push H2+ headers
                if line.starts_with("## ") {
                    result_lines.push(line)
                } else {
                    skipped_title = true
                }
            }
        }
    }
    return result_lines.join("\n")
}

// --- Template engine ---
// Replaces {{key}} placeholders with values from the vars map.
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
fn main(stdout: Stdout, fs: Fs, highlight: Highlight) {
    stdout.println("=== Basalt Static Site Generator ===")
    stdout.println("")

    // Ensure output directory exists
    if !fs.exists("output") {
        let mkdir_result = fs.mkdir("output")
        guard let _ = mkdir_result else {
            stdout.println("Error: could not create output/")
            return
        }
    }

    // Read templates
    let page_tmpl_result = fs.read_file("templates/page.html")
    guard let page_template = page_tmpl_result else {
        stdout.println("Error: could not read templates/page.html")
        return
    }
    let index_tmpl_result = fs.read_file("templates/index.html")
    guard let index_template = index_tmpl_result else {
        stdout.println("Error: could not read templates/index.html")
        return
    }

    // Read content directory
    let files_result = fs.read_dir("content")
    guard let files = files_result else {
        stdout.println("Error: could not read content/")
        return
    }

    // Filter to .md files
    let md_files = files.filter(fn(f: string) -> bool {
        let ext = fs.extension(f)
        if ext is nil { return false }
        return (ext as string) == "md"
    })

    stdout.println("Found \(md_files.length as string) markdown files")

    // First pass: read all files, parse frontmatter, collect page info
    // We store parallel arrays for: filename stem, title, date, description, draft flag
    let mut stems: [string] = []
    let mut titles: [string] = []
    let mut dates: [string] = []
    let mut descriptions: [string] = []
    let mut bodies: [string] = []

    for file in md_files {
        let stem_opt = fs.stem(file)
        if stem_opt is nil { continue }
        let stem = stem_opt as string

        let read_result = fs.read_file(fs.join("content", file))
        guard let content = read_result else {
            stdout.println("  Skipping \(file): read error")
            continue
        }

        let meta = parse_frontmatter(content)

        // Skip drafts
        if meta.contains_key("draft") {
            if meta["draft"] == "true" {
                stdout.println("  Skipping \(file) (draft)")
                continue
            }
        }

        let title = if meta.contains_key("title") { meta["title"] } else { stem }
        let date = if meta.contains_key("date") { meta["date"] } else { "" }
        let desc = if meta.contains_key("description") { meta["description"] } else { "" }

        let md_content = strip_frontmatter(content)
        let body_html = md_to_html(md_content, highlight)

        stems.push(stem)
        titles.push(title)
        dates.push(date)
        descriptions.push(desc)
        bodies.push(body_html)
    }

    // Sort pages by date (ascending for docs order)
    // Build date-sorted indices using simple insertion sort (small N)
    let mut sorted_indices: [i64] = []
    for idx in 0..stems.length {
        sorted_indices.push(idx)
    }
    // Bubble sort ascending by date string (YYYY-MM-DD sorts lexicographically)
    let mut swapped = true
    while swapped {
        swapped = false
        for j in 0..sorted_indices.length - 1 {
            let a = sorted_indices[j]
            let b = sorted_indices[j + 1]
            // Ascending: if date[a] > date[b], swap
            if dates[a] > dates[b] {
                sorted_indices[j] = b
                sorted_indices[j + 1] = a
                swapped = true
            }
        }
    }

    // Build navigation in chapter order from sorted indices
    let mut nav_parts: [string] = []
    nav_parts.push("<a href=\"index.html\">Home</a>")
    for si in sorted_indices {
        if stems[si] != "index" {
            nav_parts.push("<a href=\"" + stems[si] + ".html\">" + escape_html(titles[si]) + "</a>")
        }
    }
    let nav_html = nav_parts.join(" ")

    // Build a list of non-index sorted indices for prev/next navigation
    let mut content_order: [i64] = []
    for si in sorted_indices {
        if stems[si] != "index" {
            content_order.push(si)
        }
    }

    // Generate each content page (skip index — handled separately below)
    let mut page_count = 0
    for idx in 0..stems.length {
        if stems[idx] == "index" { continue }
        let mut vars: [string: string] = {}
        vars["title"] = titles[idx]
        vars["date"] = dates[idx]
        vars["description"] = descriptions[idx]
        vars["body"] = bodies[idx]
        vars["nav"] = nav_html

        // Compute prev/next from content_order
        let mut prev_html = ""
        let mut next_html = ""
        for ci in 0..content_order.length {
            if content_order[ci] == idx {
                if ci > 0 {
                    let pi = content_order[ci - 1]
                    prev_html = "<a href=\"" + stems[pi] + ".html\" class=\"prev\">" + escape_html(titles[pi]) + "</a>"
                }
                if ci + 1 < content_order.length {
                    let ni = content_order[ci + 1]
                    next_html = "<a href=\"" + stems[ni] + ".html\" class=\"next\">" + escape_html(titles[ni]) + "</a>"
                }
                break
            }
        }
        vars["prev"] = prev_html
        vars["next"] = next_html

        let page_html = apply_template(page_template, vars)
        let out_path = fs.join("output", stems[idx] + ".html")
        let write_result = fs.write_file(out_path, page_html)
        match write_result {
            !err => stdout.println("  Error writing \(out_path): \(err)")
            _ => {
                stdout.println("  Generated \(out_path)")
                page_count = page_count + 1
            }
        }
    }

    // Build the page listing HTML
    let mut listing_html = "<ul class=\"chapters\">\n"
    let mut chapter_num = 1
    for si in sorted_indices {
        // Skip the index page itself from the listing
        if stems[si] == "index" { continue }
        listing_html = listing_html + "<li>\n"
        listing_html = listing_html + "  <a href=\"" + stems[si] + ".html\" class=\"chapter-link\">\n"
        listing_html = listing_html + "    <span class=\"num\">" + (chapter_num as string) + "</span>\n"
        listing_html = listing_html + "    <div class=\"chapter-info\">\n"
        listing_html = listing_html + "      <span class=\"chapter-title\">" + escape_html(titles[si]) + "</span>\n"
        listing_html = listing_html + "      <div class=\"date\">" + escape_html(dates[si]) + "</div>\n"
        listing_html = listing_html + "      <div class=\"desc\">" + escape_html(descriptions[si]) + "</div>\n"
        listing_html = listing_html + "    </div>\n"
        listing_html = listing_html + "  </a>\n"
        listing_html = listing_html + "</li>\n"
        chapter_num = chapter_num + 1
    }
    listing_html = listing_html + "</ul>\n"

    // Build index page: combine index.md body (if exists) with listing
    let mut index_body = ""
    let mut index_title = "Home"
    let mut index_desc = ""
    let mut index_date = ""
    for idx in 0..stems.length {
        if stems[idx] == "index" {
            index_body = bodies[idx]
            index_title = titles[idx]
            index_desc = descriptions[idx]
            index_date = dates[idx]
            break
        }
    }
    index_body = index_body + "<h2>Chapters</h2>\n" + listing_html

    // Apply index template
    let mut index_vars: [string: string] = {}
    index_vars["title"] = index_title
    index_vars["description"] = index_desc
    index_vars["date"] = index_date
    index_vars["body"] = index_body
    index_vars["nav"] = nav_html

    let index_html = apply_template(index_template, index_vars)
    let index_write = fs.write_file("output/index.html", index_html)
    match index_write {
        !err => stdout.println("  Error writing index: \(err)")
        _ => {
            stdout.println("  Generated output/index.html")
            page_count = page_count + 1
        }
    }

    stdout.println("")
    stdout.println("Build complete: \(page_count as string) pages generated")
}
