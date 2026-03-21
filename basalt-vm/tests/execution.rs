/// End-to-end execution tests.
/// These tests compile Basalt source and execute it, verifying output.
use basalt_core;
use basalt_vm::VM;

fn run_and_capture(source: &str) -> Result<Vec<String>, String> {
    let program = basalt_core::compile(source)?;
    let mut vm = VM::new(program);
    vm.run()?;
    Ok(vm.captured_output.clone())
}

fn run_expect_output(source: &str, expected: &[&str]) {
    match run_and_capture(source) {
        Ok(output) => {
            assert_eq!(
                output,
                expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                "Output mismatch"
            );
        }
        Err(e) => panic!("Execution failed: {}", e),
    }
}

fn run_expect_panic(source: &str, msg_contains: &str) {
    match run_and_capture(source) {
        Ok(output) => panic!("Expected panic but got output: {:?}", output),
        Err(e) => {
            assert!(
                e.contains(msg_contains),
                "Expected error containing '{}', got: {}",
                msg_contains,
                e
            );
        }
    }
}

fn run_expect_compile_error(source: &str) {
    let result = basalt_core::compile(source);
    assert!(
        result.is_err(),
        "Expected compile error but compilation succeeded"
    );
}

// ==================== Basic Output ====================

#[test]
fn test_hello_world() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("Hello, World!")
}
"#,
        &["Hello, World!"],
    );
}

#[test]
fn test_multiple_prints() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("line 1")
    stdout.println("line 2")
    stdout.println("line 3")
}
"#,
        &["line 1", "line 2", "line 3"],
    );
}

// ==================== Integer Arithmetic ====================

#[test]
fn test_int_addition() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((10 + 3) as string)
}
"#,
        &["13"],
    );
}

#[test]
fn test_int_subtraction() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((10 - 3) as string)
}
"#,
        &["7"],
    );
}

#[test]
fn test_int_multiplication() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((10 * 3) as string)
}
"#,
        &["30"],
    );
}

#[test]
fn test_int_division() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((10 / 3) as string)
}
"#,
        &["3"],
    );
}

#[test]
fn test_int_modulo() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((10 % 3) as string)
}
"#,
        &["1"],
    );
}

#[test]
fn test_int_power() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((2 ** 10) as string)
}
"#,
        &["1024"],
    );
}

#[test]
fn test_int_negation() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((-42) as string)
}
"#,
        &["-42"],
    );
}

// ==================== Float Arithmetic ====================

#[test]
fn test_float_addition() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((1.5 + 2.5) as string)
}
"#,
        &["4.0"],
    );
}

#[test]
fn test_float_division() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((10.0 / 3.0) as string)
}
"#,
        &["3.3333333333333335"],
    );
}

// ==================== Boolean Operations ====================

#[test]
fn test_bool_and() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((true && false) as string)
    stdout.println((true && true) as string)
}
"#,
        &["false", "true"],
    );
}

#[test]
fn test_bool_or() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((false || false) as string)
    stdout.println((false || true) as string)
}
"#,
        &["false", "true"],
    );
}

#[test]
fn test_bool_not() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((!true) as string)
    stdout.println((!false) as string)
}
"#,
        &["false", "true"],
    );
}

// ==================== Comparisons ====================

#[test]
fn test_int_comparisons() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((1 < 2) as string)
    stdout.println((2 < 1) as string)
    stdout.println((1 <= 1) as string)
    stdout.println((1 == 1) as string)
    stdout.println((1 != 2) as string)
    stdout.println((2 > 1) as string)
    stdout.println((2 >= 2) as string)
}
"#,
        &["true", "false", "true", "true", "true", "true", "true"],
    );
}

// ==================== String Operations ====================

#[test]
fn test_string_concat() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello" + " " + "world")
}
"#,
        &["hello world"],
    );
}

#[test]
fn test_string_length() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello".length as string)
}
"#,
        &["5"],
    );
}

#[test]
fn test_string_upper_lower() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello".upper())
    stdout.println("HELLO".lower())
}
"#,
        &["HELLO", "hello"],
    );
}

#[test]
fn test_string_contains() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello world".contains("world") as string)
    stdout.println("hello world".contains("xyz") as string)
}
"#,
        &["true", "false"],
    );
}

#[test]
fn test_string_starts_ends_with() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello".starts_with("hel") as string)
    stdout.println("hello".ends_with("llo") as string)
    stdout.println("hello".starts_with("xyz") as string)
}
"#,
        &["true", "true", "false"],
    );
}

#[test]
fn test_string_trim() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("  hello  ".trim())
}
"#,
        &["hello"],
    );
}

#[test]
fn test_string_replace() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello world".replace("world", "basalt"))
}
"#,
        &["hello basalt"],
    );
}

#[test]
fn test_string_repeat() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("ab".repeat(3))
}
"#,
        &["ababab"],
    );
}

// ==================== Variables ====================

#[test]
fn test_let_immutable() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 42
    stdout.println(x as string)
}
"#,
        &["42"],
    );
}

#[test]
fn test_let_mutable() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut x = 1
    x = 2
    stdout.println(x as string)
}
"#,
        &["2"],
    );
}

// ==================== Control Flow ====================

#[test]
fn test_if_true() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    if true {
        stdout.println("yes")
    }
}
"#,
        &["yes"],
    );
}

#[test]
fn test_if_false() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    if false {
        stdout.println("yes")
    }
    stdout.println("done")
}
"#,
        &["done"],
    );
}

#[test]
fn test_if_else_branch() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 3
    if x > 5 {
        stdout.println("big")
    } else {
        stdout.println("small")
    }
}
"#,
        &["small"],
    );
}

#[test]
fn test_while_loop() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut i = 0
    while i < 3 {
        stdout.println(i as string)
        i = i + 1
    }
}
"#,
        &["0", "1", "2"],
    );
}

#[test]
fn test_for_range() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    for i in 0..3 {
        stdout.println(i as string)
    }
}
"#,
        &["0", "1", "2"],
    );
}

#[test]
fn test_loop_break() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut i = 0
    loop {
        if i >= 3 {
            break
        }
        stdout.println(i as string)
        i = i + 1
    }
}
"#,
        &["0", "1", "2"],
    );
}

#[test]
fn test_while_continue() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut i = 0
    while i < 5 {
        i = i + 1
        if i == 3 {
            continue
        }
        stdout.println(i as string)
    }
}
"#,
        &["1", "2", "4", "5"],
    );
}

// ==================== Functions ====================

#[test]
fn test_function_return() {
    run_expect_output(
        r#"
fn double(x: i64) -> i64 {
    return x * 2
}

fn main(stdout: Stdout) {
    stdout.println(double(21) as string)
}
"#,
        &["42"],
    );
}

#[test]
fn test_recursion() {
    run_expect_output(
        r#"
fn factorial(n: i64) -> i64 {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

fn main(stdout: Stdout) {
    stdout.println(factorial(5) as string)
}
"#,
        &["120"],
    );
}

// ==================== Arrays ====================

#[test]
fn test_array_creation() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let arr = [10, 20, 30]
    stdout.println(arr.length as string)
}
"#,
        &["3"],
    );
}

#[test]
fn test_array_index() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let arr = [10, 20, 30]
    stdout.println(arr[0] as string)
    stdout.println(arr[1] as string)
    stdout.println(arr[2] as string)
}
"#,
        &["10", "20", "30"],
    );
}

#[test]
fn test_array_negative_index() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let arr = [10, 20, 30]
    stdout.println(arr[-1] as string)
}
"#,
        &["30"],
    );
}

#[test]
fn test_array_push_pop() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut arr = [1, 2, 3]
    arr.push(4)
    stdout.println(arr.length as string)
    let last = arr.pop()
    stdout.println(last as string)
    stdout.println(arr.length as string)
}
"#,
        &["4", "4", "3"],
    );
}

#[test]
fn test_array_reference_semantics() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut a = [1, 2, 3]
    let mut b = a
    b.push(4)
    stdout.println(a.length as string)
}
"#,
        &["4"],
    );
}

#[test]
fn test_array_clone() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let a = [1, 2, 3]
    let mut b = a.clone()
    b.push(4)
    stdout.println(a.length as string)
    stdout.println(b.length as string)
}
"#,
        &["3", "4"],
    );
}

#[test]
fn test_array_join() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let arr = [1, 2, 3]
    stdout.println(arr.join(", "))
}
"#,
        &["1, 2, 3"],
    );
}

// ==================== Maps ====================

#[test]
fn test_map_creation() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let m = {"a": 1, "b": 2, "c": 3}
    stdout.println(m.length as string)
}
"#,
        &["3"],
    );
}

#[test]
fn test_map_contains_key() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let m = {"x": 10, "y": 20}
    stdout.println(m.contains_key("x") as string)
    stdout.println(m.contains_key("z") as string)
}
"#,
        &["true", "false"],
    );
}

// ==================== Type Conversions ====================

#[test]
fn test_int_to_string() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println(42 as string)
}
"#,
        &["42"],
    );
}

#[test]
fn test_float_to_string() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println(3.14 as string)
}
"#,
        &["3.14"],
    );
}

#[test]
fn test_bool_to_string() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println(true as string)
    stdout.println(false as string)
}
"#,
        &["true", "false"],
    );
}

#[test]
fn test_int_to_float() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 42 as f64
    stdout.println((x + 0.5) as string)
}
"#,
        &["42.5"],
    );
}

// ==================== Structs ====================

#[test]
fn test_struct_fields() {
    run_expect_output(
        r#"
type Point {
    x: f64
    y: f64
}

fn main(stdout: Stdout) {
    let p = Point { x: 1.5, y: 2.5 }
    stdout.println(p.x as string)
    stdout.println(p.y as string)
}
"#,
        &["1.5", "2.5"],
    );
}

// ==================== Enums ====================

#[test]
fn test_enum_match() {
    run_expect_output(
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
        &["42"],
    );
}

// ==================== Error Handling ====================

#[test]
fn test_division_by_zero_panic() {
    run_expect_panic(
        r#"
fn main() {
    let x = 1 / 0
}
"#,
        "division by zero",
    );
}

#[test]
fn test_overflow_panic() {
    run_expect_panic(
        r#"
fn main() {
    let x = 9223372036854775807
    let y = x + 1
}
"#,
        "overflow",
    );
}

#[test]
fn test_panic_builtin() {
    run_expect_panic(
        r#"
fn main() {
    panic("something went wrong")
}
"#,
        "something went wrong",
    );
}

// ==================== Compile Errors ====================

#[test]
fn test_no_implicit_conversion() {
    run_expect_compile_error(
        r#"
fn main() {
    let x = 1 + 2.0
}
"#,
    );
}

#[test]
fn test_if_requires_bool() {
    run_expect_compile_error(
        r#"
fn main() {
    if 0 { return }
}
"#,
    );
}

#[test]
fn test_immutable_reassignment() {
    run_expect_compile_error(
        r#"
fn main() {
    let x = 42
    x = 43
}
"#,
    );
}

#[test]
fn test_missing_main() {
    run_expect_compile_error(
        r#"
fn helper() { return }
"#,
    );
}

#[test]
fn test_break_outside_loop() {
    run_expect_compile_error(
        r#"
fn main() { break }
"#,
    );
}

#[test]
fn test_continue_outside_loop() {
    run_expect_compile_error(
        r#"
fn main() { continue }
"#,
    );
}

#[test]
fn test_wrong_arg_types() {
    run_expect_compile_error(
        r#"
fn add(a: i64, b: i64) -> i64 { return a + b }
fn main() { add("a", "b") }
"#,
    );
}

#[test]
fn test_wrong_arg_count() {
    run_expect_compile_error(
        r#"
fn add(a: i64, b: i64) -> i64 { return a + b }
fn main() { add(1) }
"#,
    );
}

#[test]
fn test_undefined_variable() {
    run_expect_compile_error(
        r#"
fn main() { let x = y }
"#,
    );
}

#[test]
fn test_boolean_arithmetic_error() {
    run_expect_compile_error(
        r#"
fn main() { let x = true + false }
"#,
    );
}

#[test]
fn test_while_requires_bool() {
    run_expect_compile_error(
        r#"
fn main() {
    while 1 { break }
}
"#,
    );
}

// ==================== Extended Tests ====================

#[test]
fn test_fibonacci_recursive() {
    run_expect_output(
        r#"
fn fib(n: i64) -> i64 {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}
fn main(stdout: Stdout) {
    stdout.println(fib(10) as string)
}
"#,
        &["55"],
    );
}

#[test]
fn test_string_methods_comprehensive() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println("hello world".upper())
    stdout.println("HELLO".lower())
    stdout.println("  trim  ".trim())
    stdout.println("abc".repeat(3))
    stdout.println("hello-world".replace("-", "_"))
    stdout.println("hello".starts_with("hel") as string)
    stdout.println("hello".ends_with("xyz") as string)
    stdout.println("hello".contains("ell") as string)
    stdout.println("hello".char_at(1))
    stdout.println("hello".substring(1, 3))
}
"#,
        &[
            "HELLO WORLD",
            "hello",
            "trim",
            "abcabcabc",
            "hello_world",
            "true",
            "false",
            "true",
            "e",
            "ell",
        ],
    );
}

#[test]
fn test_for_loop_array() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let arr = [10, 20, 30]
    let mut sum = 0
    for val in arr {
        sum = sum + val
    }
    stdout.println(sum as string)
}
"#,
        &["60"],
    );
}

#[test]
fn test_match_integers() {
    run_expect_output(
        r#"
fn classify(n: i64) -> string {
    match n {
        0 => return "zero"
        1 => return "one"
        _ => return "many"
    }
    return ""
}
fn main(stdout: Stdout) {
    stdout.println(classify(0))
    stdout.println(classify(1))
    stdout.println(classify(99))
}
"#,
        &["zero", "one", "many"],
    );
}

#[test]
fn test_match_strings() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let cmd = "go"
    match cmd {
        "stop" => stdout.println("stopping")
        "go" => stdout.println("going")
        _ => stdout.println("unknown")
    }
}
"#,
        &["going"],
    );
}

#[test]
fn test_enum_variant_match() {
    run_expect_output(
        r#"
type Result {
    Ok(i64)
    Err(string)
}
fn main(stdout: Stdout) {
    let r = Result.Ok(42)
    match r {
        Result.Ok(n) => stdout.println(n as string)
        Result.Err(msg) => stdout.println(msg)
    }
    let e = Result.Err("fail")
    match e {
        Result.Ok(n) => stdout.println(n as string)
        Result.Err(msg) => stdout.println(msg)
    }
}
"#,
        &["42", "fail"],
    );
}

#[test]
fn test_struct_field_assignment() {
    run_expect_output(
        r#"
type Point { x: f64, y: f64 }
fn main(stdout: Stdout) {
    let mut p = Point { x: 1.0, y: 2.0 }
    p.x = 10.0
    stdout.println(p.x as string)
}
"#,
        &["10.0"],
    );
}

#[test]
fn test_guard_statement() {
    run_expect_output(
        r#"
fn check(x: i64, stdout: Stdout) {
    guard x > 0 else {
        stdout.println("negative")
        return
    }
    stdout.println("positive")
}
fn main(stdout: Stdout) {
    check(5, stdout)
    check(-1, stdout)
}
"#,
        &["positive", "negative"],
    );
}

#[test]
fn test_lambda_callback() {
    run_expect_output(
        r#"
fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
    return f(x)
}
fn main(stdout: Stdout) {
    let triple = fn(x: i64) -> i64 { return x * 3 }
    stdout.println(apply(triple, 7) as string)
}
"#,
        &["21"],
    );
}

#[test]
fn test_nested_if() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 15
    if x > 10 {
        if x > 20 {
            stdout.println("very big")
        } else {
            stdout.println("big")
        }
    } else {
        stdout.println("small")
    }
}
"#,
        &["big"],
    );
}

#[test]
fn test_array_out_of_bounds_panic() {
    run_expect_panic(
        r#"
fn main() {
    let arr = [1, 2, 3]
    let x = arr[5]
}
"#,
        "out of bounds",
    );
}

#[test]
fn test_modulo_by_zero_panic() {
    run_expect_panic(
        r#"
fn main() {
    let x = 10 % 0
}
"#,
        "modulo by zero",
    );
}

#[test]
fn test_string_comparison() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println(("abc" == "abc") as string)
    stdout.println(("abc" != "def") as string)
    stdout.println(("abc" < "def") as string)
}
"#,
        &["true", "true", "true"],
    );
}

#[test]
fn test_power_right_associative() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    // 2 ** 3 ** 2 = 2 ** 9 = 512
    stdout.println((2 ** 3 ** 2) as string)
}
"#,
        &["512"],
    );
}

#[test]
fn test_bitwise_operations() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println((5 & 3) as string)
    stdout.println((5 | 3) as string)
    stdout.println((5 ^ 3) as string)
    stdout.println((1 << 4) as string)
    stdout.println((16 >> 2) as string)
}
"#,
        &["1", "7", "6", "16", "4"],
    );
}

#[test]
fn test_error_result_match() {
    run_expect_output(
        r#"
fn divide(a: f64, b: f64) -> f64!string {
    if b == 0.0 { return !("div by zero") }
    return a / b
}
fn main(stdout: Stdout) {
    match divide(10.0, 2.0) {
        !err => stdout.println("err: " + err)
        val => stdout.println(val as string)
    }
    match divide(10.0, 0.0) {
        !err => stdout.println("err: " + err)
        val => stdout.println(val as string)
    }
}
"#,
        &["5.0", "err: div by zero"],
    );
}

#[test]
fn test_multiple_functions() {
    run_expect_output(
        r#"
fn add(a: i64, b: i64) -> i64 { return a + b }
fn mul(a: i64, b: i64) -> i64 { return a * b }
fn compute(x: i64) -> i64 { return add(mul(x, x), x) }
fn main(stdout: Stdout) {
    stdout.println(compute(5) as string)
}
"#,
        &["30"],
    );
}

#[test]
fn test_map_set_and_get() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut m = {"a": 1, "b": 2}
    m["c"] = 3
    stdout.println(m.length as string)
    stdout.println(m["c"] as string)
}
"#,
        &["3", "3"],
    );
}

#[test]
fn test_array_sort_reverse() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut arr = [3, 1, 4, 1, 5]
    arr.sort()
    stdout.println(arr.join(","))
    arr.reverse()
    stdout.println(arr.join(","))
}
"#,
        &["1,1,3,4,5", "5,4,3,1,1"],
    );
}

// ==================== Forward References ====================

#[test]
fn test_forward_function_reference() {
    run_expect_output(
        r#"
fn first(n: i64) -> i64 { return second(n) + 1 }
fn second(n: i64) -> i64 { return n * 2 }
fn main(stdout: Stdout) { stdout.println(first(5) as string) }
"#,
        &["11"],
    );
}

#[test]
fn test_mutual_recursion() {
    run_expect_output(
        r#"
fn is_even(n: i64) -> bool {
    if n == 0 { return true }
    return is_odd(n - 1)
}
fn is_odd(n: i64) -> bool {
    if n == 0 { return false }
    return is_even(n - 1)
}
fn main(stdout: Stdout) {
    stdout.println(is_even(10) as string)
    stdout.println(is_odd(7) as string)
}
"#,
        &["true", "true"],
    );
}

// ==================== Guard Divergence ====================

#[test]
fn test_guard_divergence_enforced() {
    run_expect_compile_error(
        r#"
fn main() {
    guard true else { let x = 1 }
}
"#,
    );
}

#[test]
fn test_guard_with_return_ok() {
    run_expect_output(
        r#"
fn check(x: i64, stdout: Stdout) {
    guard x > 0 else {
        stdout.println("neg")
        return
    }
    stdout.println("pos")
}
fn main(stdout: Stdout) {
    check(5, stdout)
    check(-1, stdout)
}
"#,
        &["pos", "neg"],
    );
}

// ==================== Literal Range Checks ====================

#[test]
fn test_u8_literal_in_range() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let b: u8 = 255
    stdout.println(b as string)
}
"#,
        &["255"],
    );
}

#[test]
fn test_u8_literal_out_of_range() {
    run_expect_compile_error(
        r#"
fn main() {
    let b: u8 = 256
}
"#,
    );
}

#[test]
fn test_i8_negative_literal_in_range() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let b: i8 = -128
    stdout.println(b as string)
}
"#,
        &["-128"],
    );
}

#[test]
fn test_i8_negative_literal_out_of_range() {
    run_expect_compile_error(
        r#"
fn main() {
    let b: i8 = -129
}
"#,
    );
}

// ==================== Reserved Keywords ====================

#[test]
fn test_async_reserved() {
    run_expect_compile_error(
        r#"
fn main() {
    let async = 1
}
"#,
    );
}

// ==================== Optional as String ====================

#[test]
fn test_optional_as_string_nil() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = "hello".find("xyz")
    stdout.println(x as string)
}
"#,
        &["nil"],
    );
}

#[test]
fn test_optional_as_string_value() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = "hello world".find("world")
    stdout.println(x as string)
}
"#,
        &["6"],
    );
}

// ==================== String Sort ====================

#[test]
fn test_string_array_sort() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut arr = ["banana", "apple", "cherry"]
    arr.sort()
    stdout.println(arr.join(", "))
}
"#,
        &["apple, banana, cherry"],
    );
}

// ==================== Array Bounds ====================

#[test]
fn test_array_insert_out_of_bounds() {
    run_expect_panic(
        r#"
fn main() {
    let mut arr = [1, 2, 3]
    arr.insert(99, 42)
}
"#,
        "insert index 99 out of bounds",
    );
}

#[test]
fn test_array_remove_out_of_bounds() {
    run_expect_panic(
        r#"
fn main() {
    let mut arr = [1, 2, 3]
    arr.remove(5)
}
"#,
        "remove index 5 out of bounds",
    );
}

// ==================== Nil as String ====================

#[test]
fn test_nil_as_string() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    stdout.println(nil as string)
}
"#,
        &["nil"],
    );
}

// ==================== Result Type Chain ====================

#[test]
fn test_result_try_propagation() {
    run_expect_output(
        r#"
fn step1(x: i64) -> i64!string {
    if x < 0 { return !("negative") }
    return x + 10
}
fn step2(x: i64) -> i64!string {
    let v = step1(x)?
    return v * 2
}
fn main(stdout: Stdout) {
    match step2(5) {
        !e => stdout.println("err: " + e)
        v => stdout.println(v as string)
    }
    match step2(-1) {
        !e => stdout.println("err: " + e)
        v => stdout.println(v as string)
    }
}
"#,
        &["30", "err: negative"],
    );
}

// ==================== Nested Struct Access ====================

#[test]
fn test_nested_struct_field() {
    run_expect_output(
        r#"
type Inner { val: i64 }
type Outer { inner: Inner }
fn main(stdout: Stdout) {
    let o = Outer { inner: Inner { val: 42 } }
    stdout.println(o.inner.val as string)
}
"#,
        &["42"],
    );
}

// ==================== Security Limits ====================

#[test]
fn test_string_repeat_bomb() {
    run_expect_panic(
        r#"
fn main() {
    let x = "A".repeat(999999999)
}
"#,
        "string repeat too large",
    );
}

#[test]
fn test_deep_recursion_caught() {
    // Run in a thread with extra stack space — debug builds use significant
    // Rust stack per VM frame, so the host can overflow before our limit.
    let result = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            run_and_capture(
                r#"
fn f(n: i64) -> i64 {
    return f(n + 1)
}
fn main() { f(0) }
"#,
            )
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("stack overflow"),
        "expected stack overflow error"
    );
}

// ==================== Struct Methods ====================

#[test]
fn test_instance_method() {
    run_expect_output(
        r#"
type Counter {
    count: i64
    fn increment(self: Self) -> Counter {
        return Counter { count: self.count + 1 }
    }
}
fn main(stdout: Stdout) {
    let c = Counter { count: 0 }
    let c2 = c.increment()
    let c3 = c2.increment()
    stdout.println(c3.count as string)
}
"#,
        &["2"],
    );
}

#[test]
fn test_static_method() {
    run_expect_output(
        r#"
type Point {
    x: f64
    y: f64
    fn origin() -> Point {
        return Point { x: 0.0, y: 0.0 }
    }
}
fn main(stdout: Stdout) {
    let p = Point.origin()
    stdout.println(p.x as string)
    stdout.println(p.y as string)
}
"#,
        &["0.0", "0.0"],
    );
}

#[test]
fn test_method_with_args() {
    run_expect_output(
        r#"
type Vec2 {
    x: f64
    y: f64
    fn add(self: Self, other: Vec2) -> Vec2 {
        return Vec2 { x: self.x + other.x, y: self.y + other.y }
    }
}
fn main(stdout: Stdout) {
    let a = Vec2 { x: 1.0, y: 2.0 }
    let b = Vec2 { x: 3.0, y: 4.0 }
    let c = a.add(b)
    stdout.println(c.x as string)
    stdout.println(c.y as string)
}
"#,
        &["4.0", "6.0"],
    );
}

// ==================== Integer Type Soundness ====================
// The spec says: "Integer arithmetic uses checked operations —
// overflow is a runtime panic." For narrow types, exceeding the
// type's range IS overflow.

#[test]
fn test_u8_overflow_panics() {
    run_expect_panic(
        r#"
fn main() {
    let a: u8 = 200
    let b: u8 = 200
    let c = a + b
}
"#,
        "out of range",
    );
}

#[test]
fn test_u8_subtraction_underflow_panics() {
    run_expect_panic(
        r#"
fn main() {
    let a: u8 = 10
    let b: u8 = 20
    let c = a - b
}
"#,
        "out of range",
    );
}

#[test]
fn test_i8_overflow_panics() {
    run_expect_panic(
        r#"
fn main() {
    let a: i8 = 100
    let b: i8 = 100
    let c = a + b
}
"#,
        "out of range",
    );
}

#[test]
fn test_u8_arithmetic_in_range_ok() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let a: u8 = 100
    let b: u8 = 55
    let c = a + b
    stdout.println(c as string)
}
"#,
        &["155"],
    );
}

#[test]
fn test_i16_multiply_overflow() {
    run_expect_panic(
        r#"
fn main() {
    let a: i16 = 200
    let b: i16 = 200
    let c = a * b
}
"#,
        "out of range",
    );
}

// ==================== Result Type Must Be Used ====================
// The spec says: "Errors cannot be silently ignored."

#[test]
fn test_unused_result_is_error() {
    run_expect_compile_error(
        r#"
fn might_fail() -> i64!string {
    return 42
}
fn main() {
    might_fail()
}
"#,
    );
}

// ==================== Mut Gates All Mutation ====================

#[test]
fn test_immutable_array_push_rejected() {
    run_expect_compile_error(
        r#"
fn main() {
    let arr = [1, 2, 3]
    arr.push(4)
}
"#,
    );
}

#[test]
fn test_mutable_array_push_ok() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut arr = [1, 2, 3]
    arr.push(4)
    stdout.println(arr.length as string)
}
"#,
        &["4"],
    );
}

#[test]
fn test_immutable_field_assign_rejected() {
    run_expect_compile_error(
        r#"
type Point { x: f64, y: f64 }
fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    p.x = 3.0
}
"#,
    );
}

#[test]
fn test_mutable_field_assign_ok() {
    run_expect_output(
        r#"
type Point { x: f64, y: f64 }
fn main(stdout: Stdout) {
    let mut p = Point { x: 1.0, y: 2.0 }
    p.x = 3.0
    stdout.println(p.x as string)
}
"#,
        &["3.0"],
    );
}

#[test]
fn test_immutable_index_assign_rejected() {
    run_expect_compile_error(
        r#"
fn main() {
    let arr = [1, 2, 3]
    arr[0] = 99
}
"#,
    );
}

#[test]
fn test_immutable_map_assign_rejected() {
    run_expect_compile_error(
        r#"
fn main() {
    let m = {"a": 1}
    m["b"] = 2
}
"#,
    );
}

#[test]
fn test_immutable_array_sort_rejected() {
    run_expect_compile_error(
        r#"
fn main() {
    let arr = [3, 1, 2]
    arr.sort()
}
"#,
    );
}

#[test]
fn test_immutable_array_read_ok() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let arr = [1, 2, 3]
    stdout.println(arr.length as string)
    stdout.println(arr.contains(2) as string)
    stdout.println(arr.join(","))
}
"#,
        &["3", "true", "1,2,3"],
    );
}

// ==================== Closures ====================

#[test]
fn test_closure_captures_variable() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 10
    let add_x = fn(n: i64) -> i64 { return n + x }
    stdout.println(add_x(5) as string)
    stdout.println(add_x(20) as string)
}
"#,
        &["15", "30"],
    );
}

#[test]
fn test_closure_multiple_captures() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let a = 10
    let b = 20
    let sum = fn(x: i64) -> i64 { return x + a + b }
    stdout.println(sum(5) as string)
}
"#,
        &["35"],
    );
}

#[test]
fn test_closure_as_callback() {
    run_expect_output(
        r#"
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { return f(x) }
fn main(stdout: Stdout) {
    let factor = 3
    let mul = fn(x: i64) -> i64 { return x * factor }
    stdout.println(apply(mul, 7) as string)
}
"#,
        &["21"],
    );
}

#[test]
fn test_nested_closure() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 10
    let make_adder = fn(y: i64) -> fn(i64) -> i64 {
        return fn(z: i64) -> i64 { return x + y + z }
    }
    let add15 = make_adder(5)
    stdout.println(add15(100) as string)
}
"#,
        &["115"],
    );
}

// ==================== Capture By Reference ====================

#[test]
fn test_closure_counter_by_reference() {
    run_expect_output(
        r#"
fn make_counter() -> fn() -> i64 {
    let mut count = 0
    return fn() -> i64 {
        count = count + 1
        return count
    }
}
fn main(stdout: Stdout) {
    let c = make_counter()
    stdout.println(c() as string)
    stdout.println(c() as string)
    stdout.println(c() as string)
}
"#,
        &["1", "2", "3"],
    );
}

#[test]
fn test_closure_shared_mutation() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut x = 10
    let set_x = fn(v: i64) { x = v }
    let get_x = fn() -> i64 { return x }
    stdout.println(get_x() as string)
    set_x(42)
    stdout.println(get_x() as string)
    stdout.println(x as string)
}
"#,
        &["10", "42", "42"],
    );
}

// ==================== Union Exhaustiveness ====================

#[test]
fn test_union_match_exhaustive_ok() {
    run_expect_output(
        r#"
type Val = i64 | f64 | string
fn describe(v: Val) -> string {
    match v {
        is i64 => return "int"
        is f64 => return "float"
        is string => return "string"
    }
}
fn main(stdout: Stdout) {
    stdout.println(describe(42))
}
"#,
        &["int"],
    );
}

#[test]
fn test_union_match_non_exhaustive_error() {
    run_expect_compile_error(
        r#"
type Val = i64 | f64 | string
fn main() {
    let v: Val = 42
    match v {
        is i64 => return
    }
}
"#,
    );
}

// ==================== Newline Continuation ====================

#[test]
fn test_multiline_binary_op() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 1 +
        2 +
        3
    stdout.println(x as string)
}
"#,
        &["6"],
    );
}

#[test]
fn test_multiline_method_chain() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = "  Hello World  "
        .trim()
        .lower()
    stdout.println(x)
}
"#,
        &["hello world"],
    );
}

// ==================== Identity Test ====================

#[test]
fn test_is_identity_same_object() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let a = [1, 2, 3]
    let b = a
    stdout.println((a is b) as string)
}
"#,
        &["true"],
    );
}

#[test]
fn test_is_identity_different_objects() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let a = [1, 2, 3]
    let c = [1, 2, 3]
    stdout.println((a is c) as string)
    stdout.println((a == c) as string)
}
"#,
        &["false", "true"],
    );
}

#[test]
fn test_is_identity_value_types() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let a = 42
    let b = 42
    stdout.println((a is b) as string)
}
"#,
        &["true"],
    );
}

// ==================== Variable Shadowing ====================

#[test]
fn test_shadowing_in_block() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = 1
    if true {
        let x = 2
        stdout.println(x as string)
    }
    stdout.println(x as string)
}
"#,
        &["2", "1"],
    );
}

#[test]
fn test_shadowing_in_loop() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let x = "outer"
    for i in 0..3 {
        let x = i
        stdout.println(x as string)
    }
    stdout.println(x)
}
"#,
        &["0", "1", "2", "outer"],
    );
}

// ==================== Cross-Type Forward References ====================

#[test]
fn test_forward_type_reference() {
    run_expect_output(
        r#"
type A { b: B }
type B { c: C }
type C { val: i64 }
fn main(stdout: Stdout) {
    let a = A { b: B { c: C { val: 42 } } }
    stdout.println(a.b.c.val as string)
}
"#,
        &["42"],
    );
}

// ==================== Closure Captures Loop Variable ====================

#[test]
fn test_closure_captures_loop_var() {
    run_expect_output(
        r#"
fn main(stdout: Stdout) {
    let mut fns = [fn() -> i64 { return 0 }]
    fns.pop()
    for i in 0..3 {
        fns.push(fn() -> i64 { return i })
    }
    for f in fns {
        stdout.println(f() as string)
    }
}
"#,
        &["0", "1", "2"],
    );
}
