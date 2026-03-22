pub mod ast;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod types;

pub use compiler::Program;
pub use error::{CompileError, CompileErrors};

use std::path::Path;

/// Result of a successful compilation, including source for error rendering.
pub struct CompileResult {
    pub program: Program,
    pub source: String,
    pub filename: String,
}

/// Compile source code to a Program.
pub fn compile(source: &str) -> Result<Program, String> {
    let tokens = lexer::lex(source)?;
    let ast = parser::parse(tokens)?;
    let checked = types::check(&ast)?;
    let program = compiler::compile(&checked)?;
    Ok(program)
}

/// Compile a file, returning a CompileResult with source for diagnostics.
/// On error, returns a CompileError with source location info.
pub fn compile_file_rich(path: &Path) -> Result<CompileResult, (CompileErrors, String, String)> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        let msg = format!("cannot read {}: {}", path.display(), e);
        (
            CompileErrors::single(CompileError::bare(&msg)),
            String::new(),
            path.display().to_string(),
        )
    })?;
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let tokens = lexer::lex(&source)
        .map_err(|e| (CompileErrors::single(e), source.clone(), filename.clone()))?;
    let mut ast = parser::parse(tokens)
        .map_err(|e| (CompileErrors::single(e), source.clone(), filename.clone()))?;

    // Resolve imports
    let dir = path.parent().unwrap_or(Path::new("."));
    resolve_imports(&mut ast, dir, path).map_err(|e| {
        (
            CompileErrors::single(CompileError::bare(&e)),
            source.clone(),
            filename.clone(),
        )
    })?;

    let checked = types::check(&ast).map_err(|e| (e, source.clone(), filename.clone()))?;
    let program = compiler::compile(&checked).map_err(|e| {
        (
            CompileErrors::single(CompileError::bare(&e)),
            source.clone(),
            filename.clone(),
        )
    })?;

    Ok(CompileResult {
        program,
        source,
        filename,
    })
}

/// Compile a file (resolving imports relative to file's directory).
/// Legacy API returning String errors for backward compatibility.
pub fn compile_file(path: &Path) -> Result<Program, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let tokens = lexer::lex(&source)?;
    let mut ast = parser::parse(tokens)?;

    // Resolve imports
    let dir = path.parent().unwrap_or(Path::new("."));
    resolve_imports(&mut ast, dir, path)?;

    let checked = types::check(&ast)?;
    let program = compiler::compile(&checked)?;
    Ok(program)
}

fn resolve_imports(
    program: &mut ast::Program,
    base_dir: &Path,
    entry_file: &Path,
) -> Result<(), String> {
    let mut visited = std::collections::HashSet::new();
    // Mark the entry file as visited
    let entry_canonical = entry_file
        .canonicalize()
        .unwrap_or_else(|_| entry_file.to_path_buf());
    visited.insert(entry_canonical);
    resolve_imports_recursive(program, base_dir, &mut visited)
}

fn resolve_imports_recursive(
    program: &mut ast::Program,
    base_dir: &Path,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<(), String> {
    let mut imported_items = Vec::new();

    for item in &program.items {
        if let ast::Item::Import(imp) = item {
            if imp.path.starts_with("std/") {
                continue;
            }
            let mut module_file = base_dir.to_path_buf();
            module_file.push(format!("{}.bas", imp.path));

            let module_canonical = module_file
                .canonicalize()
                .unwrap_or_else(|_| module_file.clone());

            if visited.contains(&module_canonical) {
                return Err(format!(
                    "circular import detected: '{}' is already in the import chain",
                    imp.path
                ));
            }
            visited.insert(module_canonical);

            let source = std::fs::read_to_string(&module_file)
                .map_err(|e| format!("cannot import '{}': {}", imp.path, e))?;
            let tokens = lexer::lex(&source)?;
            let mut module_ast = parser::parse(tokens)?;

            // Recursively resolve imports in the imported module
            let module_dir = module_file.parent().unwrap_or(base_dir);
            resolve_imports_recursive(&mut module_ast, module_dir, visited)?;

            let alias = imp
                .alias
                .clone()
                .unwrap_or_else(|| imp.path.rsplit('/').next().unwrap_or(&imp.path).to_string());

            const KEYWORDS: &[&str] = &[
                "let", "mut", "fn", "return", "if", "else", "match", "for", "in", "while", "loop",
                "break", "continue", "type", "guard", "import", "as", "true", "false", "nil", "is",
            ];

            if KEYWORDS.contains(&alias.as_str()) {
                return Err(format!(
                    "import '{}' derives alias '{}' which is a keyword; use 'import \"{}\" as <alias>' to provide a non-keyword name",
                    imp.path, alias, imp.path
                ));
            }

            for module_item in module_ast.items {
                imported_items.push((alias.clone(), module_item));
            }
        }
    }

    for (alias, item) in imported_items {
        program.modules.entry(alias).or_default().push(item);
    }

    Ok(())
}
