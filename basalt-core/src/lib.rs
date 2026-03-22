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
    resolve_imports(&mut ast, dir).map_err(|e| {
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
    resolve_imports(&mut ast, dir)?;

    let checked = types::check(&ast)?;
    let program = compiler::compile(&checked)?;
    Ok(program)
}

fn resolve_imports(program: &mut ast::Program, base_dir: &Path) -> Result<(), String> {
    let mut imported_items = Vec::new();

    for item in &program.items {
        if let ast::Item::Import(imp) = item {
            let module_path = if imp.path.starts_with("std/") {
                // Standard library - skip for now, handled at type-check time
                continue;
            } else {
                let mut p = base_dir.to_path_buf();
                p.push(format!("{}.bas", imp.path));
                p
            };

            let source = std::fs::read_to_string(&module_path)
                .map_err(|e| format!("cannot import '{}': {}", imp.path, e))?;
            let tokens = lexer::lex(&source)?;
            let module_ast = parser::parse(tokens)?;

            let alias = imp
                .alias
                .clone()
                .unwrap_or_else(|| imp.path.rsplit('/').next().unwrap_or(&imp.path).to_string());

            // Add module items with the alias prefix
            for module_item in module_ast.items {
                imported_items.push((alias.clone(), module_item));
            }
        }
    }

    // Store imported modules in the program
    for (alias, item) in imported_items {
        program.modules.entry(alias).or_default().push(item);
    }

    Ok(())
}
