/// Suite runner: tests each .bas file in tests/suite/ against its .expected output.
use std::path::PathBuf;

use basalt_core;
use basalt_vm::VM;

fn run_suite_test(name: &str) {
    let suite_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("suite");
    let bas_path = suite_dir.join(format!("{}.bas", name));
    let expected_path = suite_dir.join(format!("{}.expected", name));

    assert!(
        bas_path.exists(),
        "Source file not found: {}",
        bas_path.display()
    );
    assert!(
        expected_path.exists(),
        "Expected file not found: {}",
        expected_path.display()
    );

    let expected_text = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", expected_path.display(), e));
    let expected_lines: Vec<&str> = if expected_text.is_empty() {
        vec![]
    } else {
        expected_text.lines().collect()
    };

    // Check if expected output indicates a compile/runtime error.
    if expected_lines.len() == 1 && expected_lines[0].starts_with("ERROR: ") {
        // The .bas file is expected to fail — verify it does.
        let result = basalt_core::compile_file(&bas_path).and_then(|program| {
            let mut vm = VM::new(program);
            vm.run()?;
            Ok(vm.captured_output)
        });
        assert!(
            result.is_err(),
            "{}.bas: expected an error but got success",
            name
        );
        return;
    }

    let program = basalt_core::compile_file(&bas_path)
        .unwrap_or_else(|e| panic!("{}.bas: compilation failed: {}", name, e));
    let mut vm = VM::new(program);
    vm.run()
        .unwrap_or_else(|e| panic!("{}.bas: execution failed: {}", name, e));

    let actual: Vec<&str> = vm.captured_output.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        actual, expected_lines,
        "{}.bas: output mismatch\n  actual: {:?}\nexpected: {:?}",
        name, actual, expected_lines
    );
}

#[test]
fn test_hello() {
    run_suite_test("hello");
}

#[test]
fn test_arithmetic() {
    run_suite_test("arithmetic");
}

#[test]
fn test_arrays() {
    run_suite_test("arrays");
}

#[test]
fn test_control_flow() {
    run_suite_test("control_flow");
}

#[test]
fn test_enums() {
    run_suite_test("enums");
}

#[test]
fn test_error_handling() {
    run_suite_test("error_handling");
}

#[test]
fn test_fibonacci() {
    run_suite_test("fibonacci");
}

#[test]
fn test_guard() {
    run_suite_test("guard");
}

#[test]
fn test_lambdas() {
    run_suite_test("lambdas");
}

#[test]
fn test_maps() {
    run_suite_test("maps");
}

#[test]
fn test_match_patterns() {
    run_suite_test("match_patterns");
}

#[test]
fn test_strings() {
    run_suite_test("strings");
}

#[test]
fn test_structs() {
    run_suite_test("structs");
}

#[test]
fn test_type_conversions() {
    run_suite_test("type_conversions");
}
