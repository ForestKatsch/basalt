/// Integration tests for the full compilation pipeline.
use basalt_core;

fn compile_and_check(source: &str) -> Result<basalt_core::Program, String> {
    basalt_core::compile(source)
}

#[test]
fn test_hello_world() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    stdout.println("Hello, World!")
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_let_binding() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let x = 42
    stdout.println(x as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_arithmetic() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let a = 10
    let b = 3
    stdout.println((a + b) as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_if_else() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let x = 10
    if x > 5 {
        stdout.println("big")
    } else {
        stdout.println("small")
    }
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_while_loop() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let mut i = 0
    while i < 5 {
        stdout.println(i as string)
        i = i + 1
    }
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_for_range() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    for i in 0..5 {
        stdout.println(i as string)
    }
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_function_call() {
    let result = compile_and_check(
        r#"
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn main(stdout: Stdout) {
    let result = add(3, 4)
    stdout.println(result as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_string_methods() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let s = "hello world"
    stdout.println(s.upper())
    stdout.println(s.length as string)
    stdout.println(s.contains("world") as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_array() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let arr = [1, 2, 3]
    stdout.println(arr.length as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_map() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let m = {"a": 1, "b": 2}
    stdout.println(m.length as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_struct() {
    let result = compile_and_check(
        r#"
type Point {
    x: f64
    y: f64
}

fn main(stdout: Stdout) {
    let p = Point { x: 1.0, y: 2.0 }
    stdout.println(p.x as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_enum() {
    let result = compile_and_check(
        r#"
type Color { Red, Green, Blue }

fn main(stdout: Stdout) {
    let c = Color.Red
    stdout.println("ok")
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_match_enum() {
    let result = compile_and_check(
        r#"
type Option {
    Some(i64)
    None
}

fn main(stdout: Stdout) {
    let x = Option.Some(42)
    match x {
        Option.Some(val) => stdout.println(val as string)
        Option.None => stdout.println("none")
    }
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_result_type() {
    let result = compile_and_check(
        r#"
fn divide(a: f64, b: f64) -> f64!string {
    if b == 0.0 {
        return !("division by zero")
    }
    return a / b
}

fn main(stdout: Stdout) {
    let r = divide(10.0, 2.0)
    stdout.println("ok")
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_lambda() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let double = fn(x: i64) -> i64 { return x * 2 }
    stdout.println(double(21) as string)
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_string_interpolation() {
    // Use a raw string with different delimiter to avoid Rust parsing \(
    let src = "fn main(stdout: Stdout) {\n    let name = \"Basalt\"\n    stdout.println(\"Hello, \\(name)!\")\n}\n";
    let result = compile_and_check(src);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_type_alias() {
    let result = compile_and_check(
        r#"
type Numeric = i64 | f64

fn main(stdout: Stdout) {
    stdout.println("ok")
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_tuple() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let t = (1, "hello", true)
    stdout.println("ok")
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_guard() {
    let result = compile_and_check(
        r#"
fn main(stdout: Stdout) {
    let x = 5
    guard x > 0 else {
        return
    }
    stdout.println("positive")
}
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

// Type error tests - these should FAIL compilation
#[test]
fn test_type_error_int_plus_float() {
    let result = compile_and_check(
        r#"
fn main() {
    let x = 1 + 2.0
}
"#,
    );
    assert!(result.is_err(), "Should have failed: i64 + f64");
}

#[test]
fn test_type_error_if_not_bool() {
    let result = compile_and_check(
        r#"
fn main() {
    if 0 {
        return
    }
}
"#,
    );
    assert!(result.is_err(), "Should have failed: non-bool condition");
}

#[test]
fn test_type_error_assign_immutable() {
    let result = compile_and_check(
        r#"
fn main() {
    let x = 42
    x = 43
}
"#,
    );
    assert!(result.is_err(), "Should have failed: immutable assign");
}

#[test]
fn test_no_main_error() {
    let result = compile_and_check(
        r#"
fn helper() {
    return
}
"#,
    );
    assert!(result.is_err(), "Should have failed: missing main");
}

#[test]
fn test_break_outside_loop() {
    let result = compile_and_check(
        r#"
fn main() {
    break
}
"#,
    );
    assert!(result.is_err(), "Should have failed: break not in loop");
}

#[test]
fn test_type_error_wrong_arg_type() {
    let result = compile_and_check(
        r#"
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn main() {
    add("hello", "world")
}
"#,
    );
    assert!(result.is_err(), "Should have failed: wrong arg types");
}

#[test]
fn test_type_error_wrong_arg_count() {
    let result = compile_and_check(
        r#"
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn main() {
    add(1)
}
"#,
    );
    assert!(result.is_err(), "Should have failed: wrong arg count");
}

// Forward references
#[test]
fn test_forward_function_ref() {
    let result = compile_and_check(
        r#"
fn a() -> i64 { return b() }
fn b() -> i64 { return 42 }
fn main(stdout: Stdout) { stdout.println(a() as string) }
"#,
    );
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

// Match exhaustiveness
#[test]
fn test_non_exhaustive_enum_match() {
    let result = compile_and_check(
        r#"
type Color { Red, Green, Blue }
fn main() {
    let c = Color.Red
    match c {
        Color.Red => return
    }
}
"#,
    );
    assert!(result.is_err(), "Should fail: non-exhaustive match");
}

// Guard divergence
#[test]
fn test_guard_must_diverge() {
    let result = compile_and_check(
        r#"
fn main() {
    guard true else { let x = 1 }
}
"#,
    );
    assert!(result.is_err(), "Should fail: guard else doesn't diverge");
}

// Literal range checks
#[test]
fn test_u8_range_check() {
    let result = compile_and_check(
        r#"
fn main() { let x: u8 = 256 }
"#,
    );
    assert!(result.is_err(), "Should fail: 256 out of range for u8");
}

// Reserved keywords
#[test]
fn test_reserved_async() {
    let result = compile_and_check(
        r#"
fn main() { let async = 1 }
"#,
    );
    assert!(result.is_err(), "Should fail: async is reserved");
}
