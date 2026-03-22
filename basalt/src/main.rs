/// Basalt CLI - Command-line interface for the Basalt programming language.
use std::path::Path;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exit_code = match parse_and_run(&args[1..]) {
        Ok(()) => 0,
        Err(CliError::Compile) => 1,
        Err(CliError::Runtime) => 2,
    };
    process::exit(exit_code);
}

enum CliError {
    Compile,
    Runtime,
}

fn parse_and_run(args: &[String]) -> Result<(), CliError> {
    match args.first().map(String::as_str) {
        None | Some("help" | "--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some("version" | "--version" | "-v") => {
            println!("basalt {VERSION}");
            Ok(())
        }
        Some("run") => {
            let path = require_file_arg(args)?;
            let extra_args: Vec<String> = args.get(2..).unwrap_or_default().to_vec();
            run_file(path, &extra_args)
        }
        Some("check") => {
            let path = require_file_arg(args)?;
            check_file(path)
        }
        Some(cmd) => {
            eprintln!("error: unknown command '{cmd}'");
            eprintln!();
            print_help();
            Err(CliError::Compile)
        }
    }
}

/// Extracts the file argument from `[command, file, ...]`, or prints an error.
fn require_file_arg(args: &[String]) -> Result<&str, CliError> {
    match args.get(1) {
        Some(path) => Ok(path.as_str()),
        None => {
            eprintln!("error: missing file argument");
            eprintln!("usage: basalt {} <file.bas>", &args[0]);
            Err(CliError::Compile)
        }
    }
}

fn print_help() {
    println!(
        "\
basalt {VERSION}
The Basalt programming language

Usage: basalt <command> [options] [file]

Commands:
  run <file.bas>      Compile and execute a Basalt program
  check <file.bas>    Type-check a program without running it
  version             Print version information
  help                Print this help message

Options:
  --help, -h          Print help
  --version, -v       Print version"
    );
}

fn check_file(path: &str) -> Result<(), CliError> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        eprintln!("error: file '{path}' not found");
        return Err(CliError::Compile);
    }

    match basalt_core::compile_file_rich(file_path) {
        Ok(_) => Ok(()),
        Err((errs, source, filename)) => {
            eprint!("{}", errs.render_all(&source, &filename));
            Err(CliError::Compile)
        }
    }
}

fn run_file(path: &str, extra_args: &[String]) -> Result<(), CliError> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        eprintln!("error: file '{path}' not found");
        return Err(CliError::Compile);
    }

    let result = match basalt_core::compile_file_rich(file_path) {
        Ok(r) => r,
        Err((errs, source, filename)) => {
            eprint!("{}", errs.render_all(&source, &filename));
            return Err(CliError::Compile);
        }
    };

    let mut vm = basalt_vm::VM::new(result.program);
    // Fs capability scoped to file's directory
    let fs_root = file_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    vm.set_fs_root(fs_root);
    // Pass remaining CLI args (after the file path) to Env
    vm.set_env_args(extra_args.to_vec());
    match vm.run() {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("runtime error: {e}");
            Err(CliError::Runtime)
        }
    }
}
