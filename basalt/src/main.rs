/// Basalt CLI - Command-line interface for running Basalt programs.
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: basalt run <file.bas>");
        process::exit(1);
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: basalt run <file.bas>");
                process::exit(1);
            }
            let file_path = &args[2];
            run_file(file_path);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Usage: basalt run <file.bas>");
            process::exit(1);
        }
    }
}

fn run_file(path: &str) {
    let file_path = Path::new(path);
    if !file_path.exists() {
        eprintln!("Error: file '{}' not found", path);
        process::exit(1);
    }

    let program = match basalt_core::compile_file(file_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            process::exit(1);
        }
    };

    let mut vm = basalt_vm::VM::new(program);
    match vm.run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
