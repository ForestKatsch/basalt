/// Tests for error message quality: correct spans, clear messages, no internal names.
use basalt_core::error::CompileError;

/// Compile source and return the CompileError, panicking if compilation succeeds.
fn expect_error(source: &str) -> CompileError {
    match compile_to_error(source) {
        Some(e) => e,
        None => panic!("Expected compile error but compilation succeeded"),
    }
}

fn compile_to_error(source: &str) -> Option<CompileError> {
    let tokens = match basalt_core::lexer::lex(source) {
        Err(e) => return Some(e),
        Ok(t) => t,
    };
    let ast = match basalt_core::parser::parse(tokens) {
        Err(e) => return Some(e),
        Ok(a) => a,
    };
    match basalt_core::types::check(&ast) {
        Err(errs) => Some(errs.errors.into_iter().next().unwrap()),
        Ok(_) => None,
    }
}

// --- Span accuracy ---

#[test]
fn error_span_points_at_bad_token() {
    let e = expect_error("fn main(stdout: Stdout) { let x = }");
    assert!(e.span.line > 0, "span should have a line number");
    assert!(
        e.span.col > 30,
        "span should point near the }}, not the start of the line (col={})",
        e.span.col
    );
}

#[test]
fn error_span_multiline() {
    let e = expect_error("fn main(stdout: Stdout) {\n    let x: string = 42\n}");
    assert_eq!(e.span.line, 2, "error should be on line 2");
}

#[test]
fn error_span_return_type_mismatch() {
    let e = expect_error("fn foo() -> string {\n    return 42\n}\nfn main(stdout: Stdout) { }");
    assert_eq!(e.span.line, 2, "error should point at the return statement");
}

// --- No internal token names ---

#[test]
fn error_no_debug_token_names() {
    let e = expect_error("fn main(stdout: Stdout) { let x = }");
    let msg = e.message.to_lowercase();
    assert!(
        !msg.contains("rbrace"),
        "error should not contain Rust enum name 'RBrace': {}",
        e.message
    );
    assert!(
        !msg.contains("lparen"),
        "error should not contain 'LParen': {}",
        e.message
    );
    assert!(
        msg.contains("}"),
        "error should mention }} in user-friendly form: {}",
        e.message
    );
}

#[test]
fn error_expected_token_friendly() {
    let e = expect_error("fn main(stdout: Stdout) { let x: [i64 = [] }");
    let msg = e.message.to_lowercase();
    assert!(
        !msg.contains("rbracket"),
        "should not show 'RBracket': {}",
        e.message
    );
    assert!(
        msg.contains("]"),
        "should mention ] in friendly form: {}",
        e.message
    );
}

// --- Message clarity ---

#[test]
fn error_type_mismatch_shows_types() {
    let e = expect_error("fn main(stdout: Stdout) { let x = 1 + 2.0 }");
    assert!(
        e.message.contains("i64"),
        "should mention i64: {}",
        e.message
    );
    assert!(
        e.message.contains("f64"),
        "should mention f64: {}",
        e.message
    );
}

#[test]
fn error_undefined_variable_with_suggestion() {
    let e =
        expect_error("fn main(stdout: Stdout) { let hello = 1\nstdout.println(helo as string) }");
    assert!(
        e.message.contains("helo"),
        "should mention the undefined name: {}",
        e.message
    );
    assert!(
        e.message.contains("hello"),
        "should suggest the correct name: {}",
        e.message
    );
}

#[test]
fn error_arg_count_shows_expected_and_got() {
    let e = expect_error(
        "fn f(a: i64, b: i64) -> i64 { return a + b }\nfn main(stdout: Stdout) { f(1) }",
    );
    assert!(
        e.message.contains("2"),
        "should mention expected count: {}",
        e.message
    );
    assert!(
        e.message.contains("1"),
        "should mention actual count: {}",
        e.message
    );
}

#[test]
fn error_immutable_assignment() {
    let e = expect_error("fn main(stdout: Stdout) { let x = 1\nx = 2 }");
    assert!(
        e.message.contains("immutable"),
        "should mention immutability: {}",
        e.message
    );
    assert!(
        e.message.contains("x"),
        "should mention the variable name: {}",
        e.message
    );
}

#[test]
fn error_mut_method_on_immutable() {
    let e = expect_error("fn main(stdout: Stdout) { let arr = [1,2,3]\narr.push(4) }");
    assert!(
        e.message.contains("push"),
        "should mention the method: {}",
        e.message
    );
    assert!(
        e.message.contains("arr"),
        "should mention the variable: {}",
        e.message
    );
    assert!(
        e.message.contains("mut"),
        "should mention mut: {}",
        e.message
    );
}

#[test]
fn error_missing_struct_field() {
    let e =
        expect_error("type P { x: i64, y: i64 }\nfn main(stdout: Stdout) { let p = P { x: 1 } }");
    assert!(
        e.message.contains("y"),
        "should mention missing field: {}",
        e.message
    );
}

#[test]
fn error_unknown_field_with_suggestion() {
    let e = expect_error(
        "type P { x: i64 }\nfn main(stdout: Stdout) { let p = P { x: 1 }\nlet v = p.z }",
    );
    assert!(
        e.message.contains("z"),
        "should mention the bad field: {}",
        e.message
    );
    assert!(
        e.message.contains("x"),
        "should suggest closest field: {}",
        e.message
    );
}

#[test]
fn error_if_condition_not_bool() {
    let e = expect_error("fn main(stdout: Stdout) { if 42 { } }");
    assert!(
        e.message.contains("bool"),
        "should mention bool: {}",
        e.message
    );
    assert!(
        e.message.contains("i64"),
        "should mention actual type: {}",
        e.message
    );
}

#[test]
fn error_break_outside_loop() {
    let e = expect_error("fn main(stdout: Stdout) { break }");
    assert!(
        e.message.contains("outside") || e.message.contains("loop"),
        "should mention loop context: {}",
        e.message
    );
}

// --- No [line prefix in messages ---

#[test]
fn error_messages_have_no_line_prefix() {
    let cases = [
        "fn main(stdout: Stdout) { let x = }",
        "fn main(stdout: Stdout) { 1 + 2.0 }",
        "fn main(stdout: Stdout) { foo() }",
        "fn main(stdout: Stdout) { let x: string = 42 }",
    ];
    for source in cases {
        if let Some(e) = compile_to_error(source) {
            assert!(
                !e.message.contains("[line"),
                "error message should not contain '[line' prefix: {}",
                e.message
            );
        }
    }
}

// --- Soundness: array invariance ---

#[test]
fn error_array_covariance_rejected() {
    // [i64] must not be assignable to [i64 | f64] — the shared reference
    // would allow writing f64 through the wider type, corrupting the original.
    let e = expect_error(
        "fn main(stdout: Stdout) {\n    let a: [i64] = [0]\n    let mut b: [i64 | f64] = a\n}",
    );
    assert!(
        e.message.contains("type mismatch"),
        "should reject: {}",
        e.message
    );
}

#[test]
fn error_array_covariance_push_blocked() {
    // Same hole via push: can't push FuckedUpShape into [Shape] via [Shape | FuckedUpShape].
    let e = expect_error(
        "type S { V(f64) }\ntype T { V() }\nfn main(stdout: Stdout) {\n    let a: [S] = []\n    let mut b: [S | T] = a\n}",
    );
    assert!(
        e.message.contains("type mismatch"),
        "should reject: {}",
        e.message
    );
}

// --- Soundness: missing return ---

#[test]
fn error_missing_return_function() {
    let e = expect_error("fn foo() -> i64 { let x = 42 }\nfn main(stdout: Stdout) { }");
    assert!(
        e.message.contains("not all code paths return"),
        "should catch missing return: {}",
        e.message
    );
}

#[test]
fn error_missing_return_lambda() {
    let e = expect_error("fn main(stdout: Stdout) { let f = fn() -> i64 { } }");
    assert!(
        e.message.contains("not all code paths return"),
        "should catch missing return in lambda: {}",
        e.message
    );
}

#[test]
fn error_missing_return_empty_body() {
    let e = expect_error("fn foo() -> string { }\nfn main(stdout: Stdout) { }");
    assert!(
        e.message.contains("not all code paths return"),
        "empty body with return type should be caught: {}",
        e.message
    );
}

// --- Struct .length field not hijacked ---

#[test]
fn struct_length_field_works() {
    // A struct with a field called 'length' should compile and work,
    // not be intercepted by the built-in .length property.
    assert!(
        compile_to_error("type Foo { length: i64 }\nfn main(stdout: Stdout) { let f = Foo { length: 42 }\nlet x = f.length }").is_none(),
        "struct with 'length' field should compile",
    );
}
