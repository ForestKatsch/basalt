# Development Philosophy

This document defines how Basalt is built. Not aspirationally — operationally.
These are the standards the project holds itself to. Code that violates them is
not "tech debt to fix later." It is a defect.

Basalt's design promises are unusually strict: no implicit conversions, no null,
no exceptions, no hidden control flow, capability-gated I/O. A language that
makes those promises to its users must hold itself to an even higher bar
internally. The compiler is the last line of defense. If it lies, the user has
no recourse.

---

## Core Tenets

### 1. The compiler must never lie

If the type checker accepts a program, that program must not exhibit undefined
behavior, type confusion, or silent data loss at runtime. Every runtime panic
must be a condition the type system could not have prevented (integer overflow,
out-of-bounds access) — not a condition the type checker failed to catch.

This means:
- Every new language feature starts as a type-checker change. If you cannot
  type-check it, you cannot ship it.
- The type checker must reject more programs than it accepts. When in doubt,
  reject. A false positive (rejecting valid code) is annoying. A false negative
  (accepting broken code) is a trust violation.
- Match exhaustiveness must be airtight. If the compiler says "all cases
  handled," a missing case at runtime is a compiler bug.

### 2. Errors are not strings

A `Result<T, String>` error pipeline is a prototype. Robust error handling
requires:
- **Structured error types** with error codes, severity, and machine-readable
  categories.
- **Source spans** attached to every diagnostic. The user must see the exact
  characters that caused the error, in context, with a caret pointing at the
  problem.
- **Error chains.** "Type mismatch" is not a diagnostic. "Expected `i64`,
  found `f64` in argument 2 of call to `add` at line 14, column 23" is a
  diagnostic. "...which is required because `add` is declared at line 3 with
  signature `fn(i64, i64) -> i64`" is a *good* diagnostic.
- **No panic in the compiler.** The compiler processes untrusted input (user
  source code). It must never panic, regardless of what it is fed. Every
  `unwrap()` in the frontend is a crash waiting for a malformed input to find it.

The quality of error messages is not a polish task. It is a correctness task.
A user who cannot understand an error message will work around it, and their
workaround will be wrong.

### 3. If it is not tested, it is broken

Not "it might break someday." It is broken *now*, and you do not know it yet.

**Test tiers, all required:**

1. **Unit tests for invariants.** The lexer must reject invalid tokens. The
   parser must reject invalid syntax. The type checker must reject invalid
   programs. These are not integration tests — they are specification tests.
   Every rule in SPEC.md must have a corresponding test that exercises it and
   a corresponding test that violates it.

2. **End-to-end execution tests.** Compile source, run it, assert output.
   This is where most tests live today, and they are good. Maintain them.
   Every example in SPEC.md must compile and run correctly — automate this.

3. **Negative tests.** For every error the compiler can emit, there must be a
   test that triggers it and asserts the exact error message. If the message
   changes, the test breaks. This is intentional. Error messages are part of
   the user interface.

4. **Fuzz testing.** The lexer, parser, and type checker accept arbitrary byte
   sequences as input. They must not panic, leak memory, or loop forever on
   any input. Use `cargo-fuzz` or `arbitrary` to generate random source
   programs and assert that the compiler either succeeds or returns a clean
   error. This is not optional for a language that aims to be robust.

5. **Property-based tests.** For core algorithms (type unification, pattern
   exhaustiveness, integer range checking), use `proptest` or `quickcheck` to
   verify properties hold across generated inputs:
   - If `A` is a subtype of `B`, then a value of type `A` is assignable where
     `B` is expected.
   - Exhaustiveness: if a match is accepted, every possible value of the
     scrutinee type is covered by at least one arm.
   - Integer narrowing: if `as` succeeds, the value is within the target
     type's range. If it fails, it is not.

6. **The test suite directory must be automated.** The 14 `.bas` files in
   `tests/suite/` must have expected-output files and a test harness that runs
   each one and diffs the output. Manual smoke tests are not tests.

### 4. Every runtime operation must have a cost model

The user should never be surprised by performance. This means:

- **Document the cost of every built-in operation.** `string.char_at(i)` is
  O(n) because strings are UTF-8. The user must know this. If they are
  iterating with `char_at` in a loop, the language should either provide an
  O(1) alternative or make the cost visible in documentation.

- **Collection iteration must not clone.** `for item in array` should iterate
  over the array in place, not snapshot-copy the entire collection into an
  iterator. A user writing `for item in million_element_array` should not
  silently allocate a million-element copy.

- **Method dispatch must be O(1).** Linear scan through all program functions
  to find a method by name is acceptable for a prototype. A robust runtime
  uses a vtable, a method cache, or at minimum a HashMap. This is on the
  critical path of every method call.

- **Memory management must handle cycles.** Reference counting with
  `Arc<RefCell>` cannot collect cycles. A closure that captures a struct that
  holds the closure will leak. The options are: (a) add a cycle collector,
  (b) add a tracing GC, or (c) document the limitation and provide a way to
  break cycles. Option (c) is honest. Pretending the problem does not exist
  is not.

### 5. Capabilities must be real

Basalt's capability model is its most distinctive feature. A language that
promises "programs cannot perform I/O unless the host grants it" must deliver
on that promise completely, or it must not make the promise at all.

Today, capabilities are integer markers. That is a stub. A real capability
system requires:

- **Capability objects, not markers.** A `Stdout` capability should be a
  first-class value that can be passed, stored, and — critically — *not*
  forged. If the user can construct a `Stdout` by any means other than
  receiving it from the host, the capability model is broken.

- **Attenuation.** The host should be able to grant a restricted capability:
  a `Stdout` that only allows `println` but not raw `print`, or a file handle
  that only allows reads, not writes. This is what separates capability
  security from access-control lists.

- **No ambient authority.** `panic()` is documented as the one exception. That
  exception must remain singular. No future feature should introduce ambient
  authority (unrestricted access to time, randomness, environment variables,
  etc.) without going through the capability system.

- **Host-object type checking.** When the host injects a custom capability via
  `HostObject`, the type checker must know its methods and enforce its contract.
  An untyped escape hatch defeats the purpose of a statically-typed capability
  system.

### 6. The specification is the source of truth

SPEC.md is not documentation. It is the contract. The implementation must
conform to the spec, not the other way around.

- If the implementation diverges from the spec, it is the implementation that
  is wrong — unless the spec is formally amended first.
- Every spec section must have a corresponding test suite section. The spec is
  only as good as its test coverage.
- The spec must not contain features that the implementation does not support.
  Speculative features belong in a separate design document, not in SPEC.md.
  The spec describes *what is*, not what might be.

### 7. Diagnostics are a user interface

The compiler's primary output is not bytecode. It is error messages. Most
compilations fail. The quality of the failure message determines whether the
user fixes their code or fights the compiler.

Diagnostics must:
- **Point at the source.** Line number, column number, filename, and a
  rendered snippet of the offending code with a caret or underline.
- **Explain the conflict.** Not "type mismatch" — say what was expected, what
  was found, and why.
- **Suggest fixes when possible.** "Did you mean `as f64`?" is not
  hand-holding. It is respect for the user's time.
- **Be tested.** Error message text is part of the public API. If a message
  changes, a test must break, and the change must be reviewed.

### 8. Simplicity is maintained, not achieved

Basalt is deliberately simple. No generics, no traits, no operator overloading,
no implicit returns. This simplicity is the language's greatest asset and its
most fragile property.

Every feature request must answer:
- Does this compose with existing features, or does it create a special case?
- Can a new user predict what this does without reading the docs?
- Does this make error messages harder to understand?
- Does this make the type checker more complex? By how much?

If the answer to any of these is unfavorable, the feature must justify itself
against the cost. "Other languages have it" is not justification. "Users need
it to solve problem X, and there is no existing way to solve X" is.

The language that adds every useful feature becomes every other language.

---

## Engineering Standards

### Code

- No `unwrap()` or `expect()` in the compiler frontend (lexer, parser, type
  checker, codegen). These modules process untrusted input. Use `Result`
  propagation or, where a precondition is genuinely guaranteed by prior
  validation, add a comment explaining why the unwrap is safe.
- No `todo!()` or `unimplemented!()` in any path reachable from user input.
  If a feature is not implemented, the compiler must emit a clear error
  message, not panic with a Rust backtrace.
- All match arms on the `Type`, `Expr`, `Stmt`, `Op`, and `Value` enums must
  be exhaustive. Wildcard (`_`) catch-all arms in these matches are forbidden
  — they hide missing cases when a new variant is added.
- All error paths in the VM must include context: what operation failed, what
  types were involved, and ideally where in the user's source the failing
  instruction originated.

### Process

- Every change to the language (new syntax, new type rule, new built-in
  method) must update three things atomically: SPEC.md, the implementation,
  and the test suite. A PR that changes any one without the other two is
  incomplete.
- Error messages are reviewed as carefully as code. A misleading error
  message is worse than a missing feature.
- Performance-sensitive changes must include benchmarks. "It should be faster"
  is not evidence.

### Architecture

- The pipeline stages (lex, parse, type-check, compile, execute) must remain
  cleanly separated. The type checker must not know about opcodes. The VM
  must not know about source syntax. Each stage communicates through its
  defined output type, and nothing else.
- New built-in types and methods should work through the same dispatch
  mechanism as user-defined types, to the maximum extent possible. Hardcoding
  `string.split` in the type checker and `"split"` in the VM is acceptable
  for bootstrapping; it is not acceptable as the permanent architecture. When
  a pattern is hardcoded for the third time, extract a dispatch table.
- The capability system must be designed before it is extended. Adding new
  capabilities (filesystem, network, time) without a coherent object model
  will produce an ad-hoc API that cannot be secured.

---

## What "Robust" Means for Basalt

A robust language is one where:

1. **The compiler catches your mistakes before you run the program.** Not some
   of them — all the ones it promised to catch. If the type system says "no
   null," then no program accepted by the type checker will ever encounter a
   null value at runtime. Period.

2. **When something fails, you know exactly what failed and why.** The error
   message tells you the file, the line, the expected type, the actual type,
   and where the expectation came from. Stack traces at runtime tell you
   every function on the call stack. Panics tell you what invariant was
   violated.

3. **The language does not have modes.** There is no "strict mode" vs. "sloppy
   mode." There is no "debug build" that checks things "release build" skips.
   The language behaves the same way in every context. Integer overflow always
   panics. Type checking always runs. Capabilities are always enforced.

4. **You can read the code and know what it does.** No implicit conversions
   means `a + b` tells you the types of `a` and `b`. No exceptions means
   every function that can fail shows it in the return type. No null means
   every optional value is visibly optional. The code is honest.

5. **The implementation earns the trust the design promises.** A beautifully
   designed language with a buggy compiler is worse than a messy language with
   a reliable one. The implementation is the product. Everything else is
   aspiration.

---

## Current Gaps (Honest Assessment)

These are not "nice to haves." These are the distance between what Basalt
promises and what it delivers today.

### Critical (trust violations)

- ~~**All errors are `String`.**~~ RESOLVED. Errors now carry source spans,
  render with file:line:col headers, and show the offending source line with
  a caret. The type checker returns structured `CompileError` values.
- **No cycle collection.** Closures capturing structs that hold closures will
  leak memory silently. The user has no way to know this is happening and no
  tool to diagnose it.
- **Capability system is a stub.** Capabilities are integer markers, not
  objects. They cannot be attenuated, revoked, or type-checked beyond the two
  hardcoded types (Stdout, Stdin). The promise of capability security is not
  yet delivered.
- **Match arm type unification is loose.** Mismatched arm types can silently
  fall through rather than computing a union type or rejecting the mismatch.
  This means the type checker can accept programs that produce values of
  unexpected types.

### Serious (reliability gaps)

- **No fuzz testing.** The compiler accepts arbitrary input and has never been
  fuzzed. There are almost certainly inputs that cause panics, infinite loops,
  or memory exhaustion in the frontend.
- ~~**No stack traces at runtime.**~~ RESOLVED. Runtime panics now include full
  stack traces with function names and source line numbers.
- ~~**Method dispatch is O(n) in function count.**~~ RESOLVED. Method dispatch
  now uses a HashMap for O(1) lookup.
- ~~**Iterator initialization clones the collection.**~~ RESOLVED. Iterators
  now hold Arc references to the original collection.
- ~~**`tests/suite/` is not automated.**~~ RESOLVED. All 14 .bas files have
  expected output and a test harness that runs them automatically.
- **Type narrowing is limited.** `is` only narrows in the `if` branch, not
  the `else` branch. `match` does not narrow union types through binding
  patterns.
- **`unwrap()` audit not performed.** The compiler frontend likely contains
  `unwrap()` calls on paths reachable from user input.

### Moderate (missing infrastructure)

- **No CI pipeline visible.** No GitHub Actions, no pre-commit hooks, no
  automated test runs on push.
- **No benchmarks.** No way to measure or track the performance of the
  compiler or VM over time.
- ~~**CLI is minimal.**~~ RESOLVED. CLI now supports `help`, `version`,
  `check`, and `run` commands with proper flags and exit codes.
- **No LSP server.** Editor support (tree-sitter grammar, Zed extension)
  exists but provides only syntax highlighting. No go-to-definition, no
  hover types, no inline diagnostics.
- **Math stdlib is minimal.** No trigonometry, no logarithms, no constants
  (pi, e), no random number generation (even behind a capability).
- **No file I/O capability.** `FileReader` and `FileSystem` are registered as
  type names in the type checker but have no method definitions and no VM
  support. Programs cannot read or write files.
- ~~**Error messages do not include filenames.**~~ RESOLVED. Error messages
  now include filename, line, and column.

---

## Decision Log

Record significant design decisions here with date, decision, and rationale.
Future contributors should not have to re-derive why something is the way it
is.

| Date | Decision | Rationale |
|------|----------|-----------|
| | No generics | Keeps the type system simple and error messages clear. Built-in collections cover the parameterized-type need. Revisit only if users cannot solve real problems without them. |
| | No implicit returns | Explicit `return` makes control flow visible. Every function body that produces a value says so. |
| | No semicolons | Newline-as-terminator with continuation heuristics. Reduces visual noise without ambiguity (the lexer handles all edge cases). |
| | Reference counting, not GC | Simpler implementation, deterministic destruction. Cycle collection is a known gap. |
| | `panic()` is ambient | Program termination is not I/O. It is the one operation that bypasses capability checks. |