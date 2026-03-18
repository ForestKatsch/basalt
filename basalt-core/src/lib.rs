pub mod lexer;
pub mod ast;
pub mod parser;
pub mod types;
pub mod compiler;

pub use compiler::Program;

use std::path::Path;

/// Compile source code to a Program.
pub fn compile(source: &str) -> Result<Program, String> {
    let tokens = lexer::lex(source)?;
    let ast = parser::parse(tokens)?;
    let checked = types::check(&ast)?;
    let program = compiler::compile(&checked)?;
    Ok(program)
}

/// Compile a file (resolving imports relative to file's directory).
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
            
            let alias = imp.alias.clone().unwrap_or_else(|| {
                imp.path.rsplit('/').next().unwrap_or(&imp.path).to_string()
            });
            
            // Add module items with the alias prefix
            for module_item in module_ast.items {
                imported_items.push((alias.clone(), module_item));
            }
        }
    }
    
    // Store imported modules in the program
    for (alias, item) in imported_items {
        program.modules.entry(alias).or_insert_with(Vec::new).push(item);
    }
    
    Ok(())
}
