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
    let arr = [1, 2, 3]
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
    let a = [1, 2, 3]
    let b = a
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
    let b = a.clone()
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
    let p = Point { x: 1.0, y: 2.0 }
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
    let m = {"a": 1, "b": 2}
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
    let arr = [3, 1, 4, 1, 5]
    arr.sort()
    stdout.println(arr.join(","))
    arr.reverse()
    stdout.println(arr.join(","))
}
"#,
        &["1,1,3,4,5", "5,4,3,1,1"],
    );
}
