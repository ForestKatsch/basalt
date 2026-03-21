# Basalt Language Specification

This document describes the Basalt programming language in sufficient detail to
implement a compatible compiler and runtime from scratch. It covers syntax,
semantics, the type system, the standard library, and the embedding API.

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Lexical Structure](#2-lexical-structure)
3. [Types](#3-types)
4. [Declarations](#4-declarations)
5. [Expressions](#5-expressions)
6. [Statements](#6-statements)
7. [Control Flow](#7-control-flow)
8. [Pattern Matching](#8-pattern-matching)
9. [Error Handling](#9-error-handling)
10. [Modules](#10-modules)
11. [Standard Library](#11-standard-library)
12. [Capabilities and IO](#12-capabilities-and-io)
13. [Embedding API](#13-embedding-api)
14. [Grammar Summary](#14-grammar-summary)

---

## 1. Design Principles

These are not aspirational. They define what the language is.

**Strict, static typing.** Every value has a type known at compile time. The
compiler rejects type errors. There are no implicit conversions: `1 + 2.0` is
a compile error. `if 0` is a compile error. The user must be explicit:
`(1 as f64) + 2.0`.

**Explicit over implicit.** No hidden control flow, no magic methods, no
default arguments, no user-defined operator overloading. The language defines
`+` for numeric types and strings; users cannot define new operator meanings.
If something happens, the code says so.

**Value and reference semantics.** Primitive types (`i64`, `f64`, `bool`,
`nil`), strings, tuples, and functions are value types — assignment copies
the value. Structs, arrays, and maps are reference types — assignment copies
the reference, not the data. Multiple variables can refer to the same object.
This split is the same model as JavaScript, Python, and Java.

**Errors are values.** There are no exceptions. Fallible operations return
`T!E` (a result type). The `?` operator propagates errors. `match` inspects
them. Errors cannot be silently ignored.

**Capabilities, not globals.** A Basalt program cannot perform IO unless the
host explicitly grants it. The `main` function receives capabilities as typed
parameters: `fn main(stdout: Stdout, stdin: Stdin)`. If the host does not
provide `Stdout`, the program cannot print. There is one exception: `panic()`
is a global, because it is a program-termination mechanism, not an IO
operation.

**No null.** There is no null pointer, no null reference, no implicit
absence. Use `T?` (optional type) when a value may be absent, or `nil` as
an explicit unit value.

**Homogeneous collections.** An `[i64]` array contains only `i64` values. A
`[string: f64]` map has only `string` keys and `f64` values. This is enforced
at compile time for literals and at runtime for mutations.

---

## 2. Lexical Structure

### 2.1 Source Encoding

Source files are UTF-8. The file extension is `.bas` or `.basalt`.

### 2.2 Comments

Line comments start with `//` and extend to the end of the line. There are no
block comments.

```
// This is a comment.
let x = 42  // Inline comment.
```

### 2.3 Keywords

```
let    mut    fn     return   if     else    match
for    in     while  loop     break  continue
type   guard  import as      true   false   nil
is
```

`async` and `await` are reserved for future use.

### 2.4 Identifiers

Identifiers start with a lowercase letter or underscore, followed by
alphanumerics and underscores: `[a-z_][a-zA-Z0-9_]*`.

Type identifiers start with an uppercase letter: `[A-Z][a-zA-Z0-9_]*`.

`_` (lone underscore) is a wildcard, not a valid identifier.

### 2.5 Literals

#### Integer literals

```
42          // decimal
0xFF        // hexadecimal
0b1010      // binary
```

All integer literals produce `i64` values. Integer arithmetic uses checked
operations: overflow is a runtime panic, not silent wrapping.

#### Float literals

```
3.14
1.0e10
2.5E-3
```

All float literals produce `f64` values (IEEE 754 double precision).

#### Boolean literals

```
true
false
```

#### String literals

Strings are immutable, UTF-8, and heap-allocated.

```
"hello, world"
"line 1\nline 2"
"tab\there"
"null\0byte"
"escape\e[31mred\e[0m"
```

**Escape sequences:**

| Sequence | Meaning                       |
| -------- | ----------------------------- |
| `\n`     | newline (LF)                  |
| `\t`     | tab                           |
| `\r`     | carriage return               |
| `\\`     | literal backslash             |
| `\"`     | literal double quote          |
| `\0`     | null byte                     |
| `\e`     | escape (0x1B, for ANSI codes) |
| `\(`     | begin string interpolation    |

**String interpolation:**

Expressions inside `\(...)` are evaluated and converted to strings:

```
let name = "world"
let msg = "Hello, \(name)!"           // "Hello, world!"
let expr = "2 + 2 = \(2 + 2)"         // "2 + 2 = 4"
let nested = "len: \(items.length)"    // method calls work
```

The expression inside `\(...)` can be any expression, including function
calls, field access, and arithmetic. Parentheses nest naturally:
`"\(compute(a, b))"` works correctly.

**Multiline strings (Zig-style):**

Lines starting with `\\` (after optional whitespace) are concatenated into a
single string with newlines between them:

```
let json =
    \\{
    \\    "name": "basalt",
    \\    "version": 1
    \\}
```

This produces `{\n    "name": "basalt",\n    "version": 1\n}`. The `\\`
prefix and any whitespace before it are stripped. Each line contributes its
content (everything after `\\`) followed by a newline, except the last line.

#### Nil literal

```
nil
```

Represents the absence of a value. Type: `nil`.

### 2.6 Operators

Listed by precedence (lowest to highest):

| Precedence | Operators         | Associativity | Description                 |
| ---------- | ----------------- | ------------- | --------------------------- |
| 1          | `..`              | none          | range                       |
| 2          | `\|\|`            | left          | logical OR                  |
| 3          | `&&`              | left          | logical AND                 |
| 4          | `\|`              | left          | bitwise OR                  |
| 5          | `^`               | left          | bitwise XOR                 |
| 6          | `&`               | left          | bitwise AND                 |
| 7          | `==` `!=` `is`    | left          | equality, identity          |
| 8          | `<` `<=` `>` `>=` | left          | comparison                  |
| 9          | `<<` `>>`         | left          | bit shift                   |
| 10         | `+` `-`           | left          | additive                    |
| 11         | `*` `/` `%`       | left          | multiplicative              |
| 12         | `**`              | right         | exponentiation              |
| 13         | `as` `as?`        | left          | type conversion             |
| 14         | `-` `!`           | prefix        | unary negation, logical NOT |
| 15         | `.` `()` `[]` `?` | left          | access, call, index, try    |

**Arithmetic (`+`, `-`, `*`, `/`, `%`, `**`):** Both operands must be the
same numeric type. `i64 + f64`is a type error — use`as`to convert
explicitly. Integer division truncates toward zero. Integer overflow panics.
The power operator`**`is right-associative:`2 ** 3 ** 2`equals`2 ** 9 = 512`.

**String concatenation:** The `+` operator concatenates two strings:
`"hello" + " " + "world"`. This is the only case where `+` is defined for
non-numeric types. The language does not support user-defined operator
overloading.

**Equality (`==`, `!=`):** Deep structural comparison. Both operands must be
the same type. Cross-type comparison is a compile error. Comparing with
`nil` is allowed for null checks. For compound types (structs, arrays,
maps), `==` recursively compares all contents. Two separately constructed
values with the same contents are equal:

```
[1, 2, 3] == [1, 2, 3]                              // true
Point { x: 1.0, y: 2.0 } == Point { x: 1.0, y: 2.0 }  // true
{"a": 1, "b": 2} == {"b": 2, "a": 1}                // true (order-insensitive)
```

Map equality is order-insensitive — two maps are equal if they have the
same key-value pairs, regardless of insertion order. (Maps preserve
insertion order for iteration, but order does not affect equality.)

**Function equality:** Functions are value types. Two references to the
same function definition are equal. Closures are never equal to other
closures, even if they capture the same variables — each closure creation
produces a distinct value:

```
fn add(a: i64, b: i64) -> i64 { return a + b }
let f = add
let g = add
f == g              // true (same function)

let a = fn(x: i64) -> i64 { return x + 1 }
let b = fn(x: i64) -> i64 { return x + 1 }
a == b              // false (different closure instances)
```

**Identity (`is` with expressions):** Reference identity test. `a is b`
returns `true` if `a` and `b` refer to the same object in memory. For
value types (primitives, strings, tuples, functions), `is` is equivalent
to `==`. For reference types (structs, arrays, maps), `is` checks whether
the two variables point to the same allocation:

```
let a = [1, 2, 3]
let b = a
let c = [1, 2, 3]
a is b              // true (same object)
a is c              // false (different object, same contents)
a == c              // true (structural equality)
```

`is` has three roles depending on context:

1. **Type narrowing:** `val is i64` — right-hand side is a type name.
   Narrows the variable's type in the guarded block.
2. **Enum variant test:** `val is Color.Red` — right-hand side is an
   enum variant. Tests whether the value is that specific variant.
   For data-carrying variants, `val is Option.Some` matches regardless
   of the carried data. Use `match` to destructure the data.
3. **Reference identity:** `a is b` — right-hand side is an expression.
   Tests whether two values are the same object.

```
// Enum variant test
let x = Option.Some(42)
if x is Option.Some {
    // x is the Some variant (data not destructured here)
}
if x is Option.None {
    // x is the None variant
}

// Use match to destructure data
match x {
    Option.Some(val) => use(val)
    Option.None => handle_none()
}
```

The compiler disambiguates based on whether the right-hand side is a type
name, an enum variant path, or a value expression.

**Comparison (`<`, `<=`, `>`, `>=`):** Both operands must be the same
type. Cross-type comparison is a type error.

**Logical (`&&`, `||`, `!`):** Operands must be `bool`. There is no truthiness:
`if 0` and `if ""` are type errors. `&&` and `||` short-circuit.

**Bitwise (`&`, `|`, `^`, `<<`, `>>`):** Both operands must be the same
integer type. `u8 & u8` is valid. `u8 & i32` is a type error. The result
type matches the operand type. Shift amounts are range-checked against the
operand width (0-7 for `u8`, 0-15 for `u16`, 0-31 for `i32`, 0-63 for
`i64`, etc.); out-of-range shifts panic.

**Conversion (`as`, `as?`):** Type conversion operator. See
[Type Conversions](#5-13-type-conversions).

**Range (`..`):** Creates an exclusive range: `0..10` produces integers 0
through 9. Used in `for` loops.

**Try (`?`):** Propagates errors from result types. See [Error Handling](#9-error-handling).

### 2.7 Delimiters

`(` `)` `{` `}` `[` `]`

### 2.8 Punctuation

| Token | Usage                              |
| ----- | ---------------------------------- |
| `.`   | field access, method call          |
| `,`   | separator in lists                 |
| `:`   | type annotation, map entries       |
| `->`  | return type annotation             |
| `=>`  | match arm separator                |
| `=`   | assignment, let binding            |
| `!`   | error literal prefix, logical NOT  |
| `?`   | try operator, optional type suffix |

### 2.9 Newlines

Newlines are significant as statement terminators. A statement ends at a
newline unless the line ends with an operator, open bracket, or comma that
implies continuation. Semicolons are not used.

---

## 3. Types

### 3.1 Primitive Types

#### Integer types

| Type  | Size    | Range             |
| ----- | ------- | ----------------- |
| `i8`  | 1 byte  | -128 to 127       |
| `i16` | 2 bytes | -32,768 to 32,767 |
| `i32` | 4 bytes | -2^31 to 2^31-1   |
| `i64` | 8 bytes | -2^63 to 2^63-1   |
| `u8`  | 1 byte  | 0 to 255          |
| `u16` | 2 bytes | 0 to 65,535       |
| `u32` | 4 bytes | 0 to 2^32-1       |
| `u64` | 8 bytes | 0 to 2^64-1       |

Integer literals default to `i64`. Use type annotations for other widths:

```
let x = 42              // i64 (default)
let b: u8 = 255
let id: u64 = 0xDEADBEEF
let small: i16 = -1000
```

Integer arithmetic uses checked operations — overflow panics at runtime.
Assigning a value outside a type's range is a compile error when the value
is a literal, or a runtime panic when computed.

#### Float types

| Type  | Size    | Description               |
| ----- | ------- | ------------------------- |
| `f64` | 8 bytes | IEEE 754 double-precision |

Float literals produce `f64`. There is currently one float type. `f32` may
be added in the future for FFI and GPU use cases.

#### Other primitives

| Type     | Description                            |
| -------- | -------------------------------------- |
| `bool`   | `true` or `false`                      |
| `string` | Immutable UTF-8 string, heap-allocated |
| `nil`    | Unit type, absence of value            |

**Value semantics vs reference semantics:**

Basalt has two categories of types:

- **Value types:** primitives (`i64`, `f64`, `bool`, `nil`), strings,
  tuples, and functions. Assignment copies the value. The original and
  the copy are independent.
- **Reference types:** structs, arrays, and maps. Assignment copies the
  reference, not the data. Multiple variables can refer to the same object.
  Use `.clone()` for an independent deep copy.

```
// Value type — copy
let a = 42
let mut b = a
b = 43              // a is still 42

// Reference type — shared (mut required for mutation)
let mut a = [1, 2, 3]
let mut b = a
b.push(4)           // a is now [1, 2, 3, 4] (same object)

// Explicit copy
let mut c = a.clone()
c.push(5)           // a is unaffected
```

**Value representation:**

All values occupy 8-byte register slots. Value types are stored inline
as raw bits. Reference types store a raw pointer to a heap-allocated
object. Narrow integer types are widened to 64 bits in registers and
narrowed on store.

Values are untagged — the runtime does not encode type information in the
value itself. The compiler statically knows the type of every register at
every point in the program, and each bytecode instruction encodes how to
interpret its operands. For example, `AddInt` knows both operands are
integers; `GetField` knows its operand is a struct pointer. The VM never
needs to inspect a value to determine its type.

This means:

- `i64` and `u64` use the full 64-bit range (no bits reserved for tags).
- `f64` is stored as raw IEEE 754 bits (no NaN canonicalization).
- Pointers require no masking or tag stripping on access.
- There is no runtime type tag, no NaN-boxing, and no tagged union.
  Type safety is enforced entirely by the compiler.

### 3.2 Collection Types

**Arrays** are ordered, homogeneous, growable sequences. Arrays are
reference types — assignment shares the same array:

```
let nums: [i64] = [1, 2, 3]
let empty: [string] = []
let alias = nums          // alias and nums refer to the same array
let copy = nums.clone()   // independent deep copy
```

**Maps** are ordered (by insertion), homogeneous key-value stores. Maps are
reference types — assignment shares the same map:

```
let ages: [string: i64] = {"alice": 30, "bob": 25}
let empty: [string: f64] = {}
```

Map keys can be any hashable type (integers, floats, strings, booleans).
Maps preserve insertion order. Map lookup on a missing key panics. Use
`m.get(key)` for safe access that returns `V?` (nil on missing key).

**Tuples** are fixed-size, heterogeneous, immutable sequences. Tuples are
value types — assignment copies the tuple:

```
let pair: (i64, string) = (42, "hello")
let triple = (1, true, "yes")
```

Tuple elements are accessed by index: `pair.0`, `pair.1`. Tuples cannot be
mutated.

### 3.3 Named Types (Structs)

```
type Point {
    x: f64
    y: f64
}

type Color {
    r: u8
    g: u8
    b: u8
    a: u8
}

let p = Point { x: 1.0, y: 2.0 }
p.x    // 1.0

let c = Color { r: 255, g: 128, b: 0, a: 255 }  // 4 bytes
```

Struct construction requires all fields. Fields are accessed with dot
notation. Structs are reference types — assignment shares the same struct:

```
let p = Point { x: 1.0, y: 2.0 }
let q = p              // q and p refer to the same struct
q.x = 3.0              // p.x is now 3.0 too
let r = p.clone()      // independent deep copy
```

### 3.4 Enum Types

Enums define a closed set of variants. Variants may carry associated data.

```
type Color { Red, Green, Blue }

type Option {
    Some(i64)
    None
}

type Expr {
    Num(f64)
    Add(Expr, Expr)         // tuple variant with two values
    RGB(i64, i64, i64)      // tuple variant with three values
}
```

**Construction:**

```
let c = Color.Red
let x = Option.Some(42)
let e = Expr.Add(Expr.Num(1.0), Expr.Num(2.0))
```

**Recursive types:** Enum variants may reference the enclosing type,
directly or through containers. This enables trees, ASTs, and nested data:

```
type Json {
    Null
    Bool(bool)
    Num(f64)
    Str(string)
    Arr([Json])
    Obj([string: Json])
}
```

All compound values are heap-allocated, so recursive types have no
infinite-size problem.

### 3.5 Optional Types

`T?` is syntactic sugar for the union type `T | nil`:

```
let name: string? = nil       // equivalent to: let name: string | nil = nil
let value: i64? = 42
```

`nil` is compatible with any optional type. `T?` and `T | nil` are the same
type — they are fully interchangeable. This means `(i64 | string)?` is
equivalent to `i64 | string | nil`.

Use `is nil` or `is` narrowing to unwrap optionals:

```
if name is string {
    // name is narrowed to string here
}
```

### 3.6 Result Types

`T!E` represents a value that is either a success (`T`) or an error
(`Error(E)`). It is syntactic sugar for the union type `T | Error(E)`:

```
fn parse(s: string) -> i64!string {
    if s == "" { return !("empty") }
    return s as i64
}
```

Error values are constructed with `!(value)`. The `Error(E)` wrapper type
distinguishes error values from success values of the same type — e.g.,
`string` (success) vs `Error(string)` (failure) in the same result.

The `?` operator desugars to: if the value `is Error`, return it from the
enclosing function; otherwise narrow to the success type `T`.

See [Error Handling](#9-error-handling).

### 3.7 Function Types

```
fn(i64, i64) -> i64       // function taking two i64, returning i64
fn(string) -> nil          // function taking string, returning nothing
```

Functions are first-class values.

### 3.8 Type Inheritance

A type can extend another type:

```
type FileSystem: FileReader {
    fn write(self: Self, path: string, data: string) -> nil!string
}
```

A `FileSystem` value is accepted wherever a `FileReader` is expected. Type
narrowing with `as?` tests whether a value is a subtype. `as?` returns
`T?` — the narrowed value on success, or `nil` on failure:

```
let fs: FileSystem? = reader as? FileSystem
if fs is FileSystem {
    fs.write("out.txt", data)
}
```

### 3.9 Union Types and Type Aliases

#### Inline union types

A union type `A | B` accepts values of either type A or type B:

```
let val: i64 | string = 42
let val: i64 | string = "hello"
```

Unions can have any number of members: `i64 | f64 | string | bool`.

#### Named union types (type aliases)

The `type Name = ...` syntax creates a named alias for a union (or any
other type expression):

```
type Numeric = i64 | f64
type Stringable = i64 | f64 | bool | string | nil
type JsonPrimitive = bool | f64 | string | nil
```

Named unions are interchangeable with their expansion. A `Numeric` value
is accepted wherever `i64 | f64` is expected, and vice versa.

Type aliases can also name non-union types:

```
type UserId = i64
type Headers = [string: string]
```

A type alias is transparent — it is the same type as its definition, not
a new distinct type. `UserId` and `i64` are fully interchangeable.

#### Named unions with named types

Union members can be named types (structs, enums, other aliases):

```
type Shape = Circle | Rect | Triangle

type Circle {
    radius: f64
}

type Rect {
    width: f64
    height: f64
}

type Triangle {
    base: f64
    height: f64
}

fn area(s: Shape) -> f64 {
    if s is Circle { return 3.14159 * s.radius * s.radius }
    else if s is Rect { return s.width * s.height }
    else { return 0.5 * s.base * s.height }
}
```

This provides an alternative to enums for modeling variants when each
variant has its own standalone type with fields and methods.

#### Type narrowing with `is`

The `is` operator tests whether a value is a specific type. Inside the
guarded block, the compiler narrows the variable's type:

```
fn describe(val: i64 | f64 | string) -> string {
    if val is i64 {
        // val has type i64 here
        return "integer: " + (val as string)
    } else if val is f64 {
        // val has type f64 here
        return "float: " + (val as string)
    } else {
        // val has type string here (only remaining member)
        return "string: " + val
    }
}
```

**Narrowing rules:**

1. `if val is T { ... }` — inside the block, `val` has type `T`.
2. In the `else` branch, `val`'s type is the union minus `T`. If only
   one member remains, `val` is narrowed to that type automatically.
3. After `if val is T { return }`, `val` is narrowed in all subsequent
   code (because the `if` body diverges).
4. `is` works with named types (structs, enums) and primitive types.
5. `is` is a compile-time-checked operation — the type must be a member
   of the union, or the compiler reports an error.
6. Narrowing only applies to simple variable names, not arbitrary
   expressions. `if foo.bar is T` does NOT narrow `foo.bar`.
7. The compiler does not narrow through complex control flow (e.g.,
   narrowing does not propagate through function calls or assignments).

**`is` in match patterns:**

`is T` can be used as a match pattern. The scrutinee is automatically
narrowed to type `T` inside the arm body:

```
match val {
    is i64 => val as string      // val is i64 here
    is f64 => val as string      // val is f64 here
    is string => val             // val is string here
}
```

This is consistent with `if val is T` — the same keyword, the same
narrowing behavior. The match form is preferable when testing against
multiple types.

**Exhaustiveness:** `match` with `is` patterns should be exhaustive — the
compiler warns if not all union members are covered. `if`/`else if` chains
do not require exhaustiveness, as they are sequential checks with an
implicit fallthrough.

**`is` with named union types:**

```
type Token = Keyword | Ident | Number

fn classify(t: Token) -> string {
    if t is Keyword { return "keyword: " + t.text }
    else if t is Ident { return "identifier: " + t.name }
    else { return "number: " + (t.value as string) }
}
```

#### Union type rules

1. A value of type `T` is compatible with any union containing `T`.
2. `is` narrows a union to a specific member type within a branch.
3. Unions are order-independent: `i64 | string` equals `string | i64`.
4. Duplicate types are collapsed: `i64 | i64` is `i64`.
5. Nested unions are flattened: `(i64 | f64) | string` is `i64 | f64 | string`.
6. Named union aliases are transparent — `Numeric` and `i64 | f64` are
   the same type.

### 3.10 The `nil` Type

`nil` is both a type and a value. It is the only value of type `nil`.
Functions without a `-> T` return type annotation return `nil`. `nil`
represents the absence of a meaningful value.

### 3.11 Type Compatibility Rules

1. `T` is compatible with `T` (identity).
2. `T` is compatible with `T?` (a value can be used where optional expected).
3. `nil` is compatible with `T?`.
4. `T` is compatible with `T!E` (result — a success value).
5. `Error(E)` is compatible with `T!E` (result — an error value).
6. An empty array `[]` is compatible with any array type `[T]`.
7. An empty map `{}` is compatible with any map type `[K: V]`.
8. A subtype is compatible with its parent type.

### 3.12 No Generics

Basalt does not support user-defined generic types or functions. The
built-in collection types (`[T]`, `[K: V]`, tuples) are parameterized
by the language itself. User-defined types cannot be parameterized.

This is a deliberate limitation. Generics add substantial complexity
(type parameter bounds, variance, monomorphization or type erasure) that
is not justified for Basalt's use cases. Use unions and `is` narrowing
for polymorphism.

---

## 4. Declarations

### 4.1 Variable Declarations

```
let x = 42                      // immutable, type inferred as i64
let name: string = "basalt"     // immutable, type annotated
let mut counter = 0              // mutable
```

Variables are immutable by default. The `mut` keyword allows reassignment
**and mutation**. Without `mut`, you cannot reassign the variable, assign to
its fields or indices, or call mutating methods (`push`, `pop`, `sort`, etc.)
on it. All variables must be initialized at declaration.

### 4.2 Function Declarations

```
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn greet(stdout: Stdout) {
    stdout.println("hello")
}
```

All parameters require type annotations. The return type is required if the
function returns a non-nil value. Without `-> T`, the function returns `nil`.

Functions require explicit `return` statements. The last expression in a
function body is NOT implicitly returned (unlike Rust). A function without
a `return` statement returns `nil`.

### 4.3 Type Declarations

```
type Point {
    x: f64          // field
    y: f64          // field

    fn origin() -> Point {                          // static method
        return Point { x: 0.0, y: 0.0 }
    }

    fn distance(self: Self, other: Point) -> f64 {  // instance method
        let dx = self.x - other.x
        let dy = self.y - other.y
        return math.sqrt(dx * dx + dy * dy)
    }
}
```

**Fields** are declared with `name: Type`.

**Methods** are declared with `fn` inside the type body. A method is an
instance method when its first parameter is `self: Self` (or `self: TypeName`).
Otherwise it is a static method.

**Static method call:** `Point.origin()`
**Instance method call:** `p.distance(other)`

The `Self` type alias refers to the enclosing type within method signatures.

**Enum variants** are declared as uppercase names, optionally with associated
data types in parentheses:

```
type Result {
    Ok(i64)
    Err(string)
}
```

### 4.4 Type Alias Declarations

```
type Numeric = i64 | f64
type UserId = u64
type Headers = [string: string]
type Shape = Circle | Rect | Triangle
```

A type alias introduces a name for an existing type expression. Aliases
are transparent — `Numeric` and `i64 | f64` are fully interchangeable.
They are not new distinct types.

Type aliases are particularly useful with unions to name a set of accepted
types for function parameters:

```
type Serializable = i64 | f64 | bool | string | nil

fn serialize(val: Serializable) -> string {
    if val is i64 { return val as string }
    if val is f64 { return val as string }
    if val is bool { return if val { "true" } else { "false" } }
    if val is string { return "\"" + val + "\"" }
    return "null"
}
```

### 4.5 Import Declarations

```
import "geometry"                    // imports ./geometry.bas
import "lib/utils" as helpers        // imports ./lib/utils.bas, aliased
import "std/math"                    // standard library module
```

See [Modules](#10-modules).

---

## 5. Expressions

### 5.1 Literals

Integer, float, boolean, string, nil, array, map, and tuple literals as
described in [Lexical Structure](#2-lexical-structure).

```
42                          // i64
3.14                        // f64
true                        // bool
"hello"                     // string
nil                         // nil
[1, 2, 3]                   // [i64]
{"a": 1, "b": 2}            // [string: i64]
(42, "hello")                // (i64, string)
```

### 5.2 Struct Construction

```
let p = Point { x: 1.0, y: 2.0 }
```

All fields must be provided. Field order does not matter.

### 5.3 Enum Variant Construction

```
let c = Color.Red                      // no data
let x = Option.Some(42)                // single value
let e = Expr.Add(left, right)          // tuple variant
```

### 5.4 Error Construction

```
let err = !("something went wrong")    // Error(string)
```

The `!()` syntax constructs an error value. Inside a function returning
`T!E`, `return !(value)` returns the error case.

### 5.5 Field Access

```
p.x              // struct field
items.length     // builtin property
```

`.length` is a built-in property on `string`, arrays, maps, and tuples.
It is the only built-in property. User-defined types cannot add properties
— use methods instead.

### 5.6 Method Calls

```
s.trim()                    // string method
arr.push(42)                // array method
stdout.println("hello")     // capability method
Point.origin()              // static method
p.distance(other)           // instance method
```

### 5.7 Index Access

```
arr[0]           // array index (0-based)
arr[-1]          // negative index (from end)
map["key"]       // map lookup (panics if key missing)
```

Negative indices count from the end: `arr[-1]` is the last element.
Out-of-bounds array indexing panics. Map lookup on a missing key panics.
Use `m.get(key)` for safe map access that returns `V?` (the value or
`nil` if the key is not present).

### 5.8 Function Calls

```
add(1, 2)
fibonacci(30)
```

### 5.9 Lambda Expressions

```
let double = fn(x: i64) -> i64 { return x * 2 }
let add = fn(a: i64, b: i64) -> i64 { return a + b }
```

Lambdas capture variables from their enclosing scope by reference using
shared heap cells. Mutations in the closure are visible in the enclosing
scope and vice versa. Captured narrow types (e.g., `u8`) use register-width
(8 bytes) storage internally; this is transparent to the user.

### 5.10 If Expressions

`if` is an expression that produces a value:

```
let status = if x > 0 { "positive" } else { "negative" }
```

Without `else`, the type is `nil`. With `else`, both branches must produce
compatible types. `else if` chains are supported:

```
let grade = if score >= 90 { "A" }
    else if score >= 80 { "B" }
    else if score >= 70 { "C" }
    else { "F" }
```

### 5.11 Block Expressions

A block `{ ... }` is an expression. Its value is the value of its last
expression (if any), or `nil`.

### 5.12 Assignment

```
x = 42                    // variable reassignment (requires mut)
arr[0] = 99               // index assignment (requires mut on arr)
map["key"] = value         // map entry assignment (requires mut on map)
obj.field = value          // field assignment (requires mut on obj)
```

All forms of assignment require `mut` on the target binding. This applies
to variable reassignment, field assignment, index assignment, and map entry
assignment. Mutating methods (`push`, `pop`, `sort`, etc.) also require
`mut`.

Assignment is a statement, not an expression. It does not produce a value.

For reference types, mutation is visible through all references to the
same object:

```
let mut a = [1, 2, 3]
let mut b = a
a[0] = 99         // b[0] is now 99 too (same object)
```

### 5.13 Type Conversions

The `as` operator converts a value from one type to another:

```
42 as string         // "42"
42 as f64            // 42.0
"42" as i64          // 42
3.14 as i64          // 3 (truncation toward zero)
255 as u8            // 255
```

**`as` (strict conversion):** Converts the value or panics if the
conversion is impossible. Panics on: out-of-range integer narrowing
(`300 as u8`), unparseable string conversion (`"hello" as i64`).

**`as?` (safe conversion):** Returns `T?` — the converted value on
success, or `nil` on failure:

```
"hello" as? i64      // nil (can't parse)
300 as? u8           // nil (out of range)
"42" as? i64         // 42
```

**Valid conversion pairs:**

| From        | To          | Behavior                              |
| ----------- | ----------- | ------------------------------------- |
| any integer | any integer | Range-checked; panics/nil on overflow |
| any integer | `f64`       | Widening (always succeeds)            |
| `f64`       | any integer | Truncates toward zero; range-checked  |
| any numeric | `string`    | Display representation                |
| `string`    | any numeric | Parses; panics/nil on invalid input   |
| `bool`      | `string`    | `"true"` or `"false"`                 |

Cross-width integer arithmetic is a type error. The user must convert
explicitly: `(a as i64) + b` when `a: i32` and `b: i64`.

Integer literals assigned to narrow types are range-checked at compile
time: `let b: u8 = 256` is a compile error. Variable assignments across
widths require `as`: `let b: u8 = x as u8` when `x: i64`.

---

## 6. Statements

### 6.1 Let Statements

```
let x = expr
let mut x: Type = expr
```

### 6.2 Expression Statements

Any expression can appear as a statement. Its value is discarded:

```
stdout.println("hello")
arr.push(42)
```

### 6.3 Return Statements

```
return            // returns nil
return expr       // returns value
return !(msg)     // returns error
```

---

## 7. Control Flow

### 7.1 If/Else

```
if condition {
    body
} else if other_condition {
    body
} else {
    body
}
```

The condition must be `bool`. There is no truthiness.

### 7.2 While Loops

```
while condition {
    body
}
```

### 7.3 Loop (Infinite)

```
loop {
    if done { break }
}
```

### 7.4 For-In Loops

```
for item in array { ... }              // iterate array elements
for item, index in array { ... }       // element and index
for key, value in map { ... }          // iterate map entries
for char in "hello" { ... }            // iterate string characters
for i in 0..10 { ... }                 // iterate range (exclusive end)
```

The loop variables are immutable. The optional second binding provides:

- **Arrays:** the index (`i64`). The value is primary, the index is
  secondary (you always want the value, sometimes the index).
- **Maps:** the value. The key is primary (it's the lookup handle),
  the value is secondary.

Both follow "primary first, secondary second." The semantic difference
is inherent to the data structures.

### 7.5 Break and Continue

`break` exits the innermost loop. `continue` skips to the next iteration.

### 7.6 Guard

```
guard condition else { return }
guard let value = optional_expr else { return !("missing") }
```

`guard` asserts a condition. If the condition is false (or the binding fails),
the `else` block executes. The `else` block MUST diverge (`return`, `break`,
`continue`, or `panic`).

`guard let` binds the unwrapped value into the enclosing scope (not just the
guard block). This is the key difference from `if let`.

**`guard let` unwraps:**

- `T?` (optional): binds value of type `T`, else block runs on `nil`.
- `T!E` (result): binds value of type `T`, else block runs on error.

The binding variable has the unwrapped type in the enclosing scope after
the guard statement:

```
guard let name = get_name() else { return !("no name") }
// name is `string` here, not `string?`
```

---

## 8. Pattern Matching

### 8.1 Match Expressions

```
match value {
    Pattern1 => expression1
    Pattern2 => expression2
    _ => default_expression
}
```

`match` is an expression. Each arm has a pattern, `=>`, and a body. The body
can be an expression, a block, or a return/break/continue statement.

### 8.2 Pattern Types

| Pattern                  | Matches                | Binds                  |
| ------------------------ | ---------------------- | ---------------------- |
| `42`                     | integer literal        | nothing                |
| `"hello"`                | string literal         | nothing                |
| `true` / `false`         | boolean literal        | nothing                |
| `nil`                    | nil value              | nothing                |
| `name`                   | any value              | `name`                 |
| `_`                      | any value              | nothing (wildcard)     |
| `Type.Variant`           | enum variant (no data) | nothing                |
| `Type.Variant(x)`        | enum variant (1 value) | `x`                    |
| `Type.Variant(x, y)`     | enum variant (tuple)   | `x`, `y`               |
| `!err`                   | error value            | `err`                  |
| `is T`                   | union member of type T | narrows scrutinee to T |
| `module.Type.Variant`    | cross-module enum      | nothing                |
| `module.Type.Variant(x)` | cross-module enum      | `x`                    |

### 8.3 Match on Results

Result types are matched with the `!` prefix for the error case:

```
match fallible_call() {
    value => use(value)           // success case
    !err => handle(err)           // error case
}
```

The success case binds the unwrapped value. The error case uses `!name` to
bind the error.

---

## 9. Error Handling

### 9.1 Result Types

```
fn divide(a: f64, b: f64) -> f64!string {
    if b == 0.0 { return !("division by zero") }
    return a / b
}
```

`T!E` declares a function that returns either `T` (success) or `Error(E)`
(failure). Error values are constructed with `!(value)`.

### 9.2 The `?` Operator (Try)

```
fn process(s: string) -> i64!string {
    let n = parse(s)?         // if parse returns error, propagate it
    return n * 2
}
```

`expr?` evaluates `expr`. If the result is an error, it immediately returns
that error from the current function. If successful, it unwraps the value.

The enclosing function must have a compatible result return type.

### 9.3 Panic

```
panic("fatal error")
```

`panic` terminates the program with a message and stack trace. It cannot be
caught. Use it for programming errors, not recoverable failures.

---

## 10. Modules

### 10.1 File-Based Modules

Each `.bas` file is a module. The module name is the filename stem.

```
// geometry.bas
type Point {
    x: f64
    y: f64
}

fn distance(a: Point, b: Point) -> f64 { ... }
```

### 10.2 Import Syntax

```
import "geometry"                    // import ./geometry.bas
import "lib/utils" as helpers        // import with alias
import "std/math"                    // standard library
```

Import paths are relative to the importing file. Paths starting with `std/`
refer to the standard library.

### 10.3 Qualified Access

All names from an imported module are accessed with the module qualifier:

```
import "geometry"

fn main() {
    let p = geometry.Point { x: 1.0, y: 2.0 }
    let d = geometry.distance(p, origin)
}
```

There is no unqualified import (`use` or `from ... import`). All access is
always qualified.

### 10.4 Cross-Module Types

Imported types can be constructed, pattern-matched, and used in type
annotations with qualified names:

```
import "shapes"

fn area(s: shapes.Shape) -> f64 {
    match s {
        shapes.Shape.Circle(r) => return 3.14159 * r * r
        shapes.Shape.Rect(w, h) => return w * h
    }
}
```

### 10.5 No Circular Imports

Circular imports are prohibited. If module A imports module B, module B
cannot import module A.

### 10.6 Entry Point

The entry module (the one passed to `basalt run`) must define a `main`
function. If no `main` function is found, compilation fails with:
"entry module must define a `main` function."

Library modules (imported by other modules) do not require `main`.

---

## 11. Standard Library

### 11.1 Math (`import "std/math"`)

```
import "std/math"

math.sqrt(x)       // square root -> f64
math.abs(x)        // absolute value (preserves type)
math.floor(x)      // floor (preserves type, no-op on i64)
math.ceil(x)       // ceiling (preserves type, no-op on i64)
math.round(x)      // round (preserves type, no-op on i64)
math.min(a, b)     // minimum (both must be same numeric type)
math.max(a, b)     // maximum (both must be same numeric type)
```

### 11.2 Type Conversions

Type conversions use the `as` operator, not functions. There are no
conversion functions like `string()` or `i64()`. See
[Type Conversions](#5-13-type-conversions) for the full specification.

```
42 as f64              // i64 -> f64
3.14 as i64            // f64 -> i64 (truncates toward zero)
42 as string           // any value -> string representation
"42" as i64            // string -> i64 (panics on invalid input)
"3.14" as f64          // string -> f64 (panics on invalid input)
"hello" as? i64        // nil (safe conversion)
```

**Standard library type aliases:**

```
type Signed = i8 | i16 | i32 | i64
type Unsigned = u8 | u16 | u32 | u64
type Integer = Signed | Unsigned
type Numeric = Integer | f64
type Stringable = Numeric | bool | string | nil
```

### 11.3 String Methods

| Method                    | Signature                  | Description                            |
| ------------------------- | -------------------------- | -------------------------------------- |
| `s.length`                | property -> i64            | Character count (not bytes)            |
| `s.split(sep)`            | (string) -> [string]       | Split by delimiter                     |
| `s.trim()`                | () -> string               | Strip whitespace from both ends        |
| `s.trim_start()`          | () -> string               | Strip leading whitespace               |
| `s.trim_end()`            | () -> string               | Strip trailing whitespace              |
| `s.replace(from, to)`     | (string, string) -> string | Replace all occurrences                |
| `s.find(needle)`          | (string) -> i64?           | Character index of first match, or nil |
| `s.substring(start, len)` | (i64, i64) -> string       | Extract by char index and length       |
| `s.starts_with(prefix)`   | (string) -> bool           | Prefix test                            |
| `s.ends_with(suffix)`     | (string) -> bool           | Suffix test                            |
| `s.contains(sub)`         | (string) -> bool           | Substring test                         |
| `s.upper()`               | () -> string               | Uppercase                              |
| `s.lower()`               | () -> string               | Lowercase                              |
| `s.repeat(n)`             | (i64) -> string            | Repeat N times                         |
| `s.char_at(i)`            | (i64) -> string            | Single character by index              |

`char_at` supports negative indices (`-1` = last character). `split` takes a
single delimiter string.

### 11.4 Array Methods

| Method               | Signature          | Description                        |
| -------------------- | ------------------ | ---------------------------------- |
| `arr.length`         | property -> i64    | Element count                      |
| `arr.push(val)`      | (T) -> nil         | Append element (type-checked)      |
| `arr.pop()`          | () -> T            | Remove and return last element     |
| `arr.insert(i, val)` | (i64, T) -> nil    | Insert at index (type-checked)     |
| `arr.remove(i)`      | (i64) -> nil       | Remove element at index            |
| `arr.sort()`         | () -> nil          | Sort in place (i64 or string only) |
| `arr.reverse()`      | () -> nil          | Reverse in place                   |
| `arr.join(sep)`      | (string) -> string | Join elements with separator       |
| `arr.contains(val)`  | (T) -> bool        | Element membership test            |
| `arr.clone()`        | () -> [T]          | Deep copy (independent array)      |

Mutating methods (`push`, `pop`, `sort`, `reverse`, `insert`, `remove`)
mutate the array in place. All references to the same array see the
change. Array indexing supports negative indices.

### 11.5 Map Methods

| Method                | Signature       | Description                           |
| --------------------- | --------------- | ------------------------------------- |
| `m.length`            | property -> i64 | Entry count                           |
| `m[key]`              | index -> V      | Get value by key (panics if missing)  |
| `m[key] = val`        | assignment      | Set value by key                      |
| `m.get(key)`          | (K) -> V?       | Safe lookup (nil if missing)          |
| `m.contains_key(key)` | (K) -> bool     | Key membership test                   |
| `m.keys()`            | () -> [K]       | Array of all keys (insertion order)   |
| `m.values()`          | () -> [V]       | Array of all values (insertion order) |
| `m.remove(key)`       | (K) -> nil      | Remove entry by key                   |
| `m.clone()`           | () -> [K: V]    | Deep copy (independent map)           |

### 11.6 Struct Methods

All struct types have the following built-in method:

| Method      | Signature  | Description                    |
| ----------- | ---------- | ------------------------------ |
| `s.clone()` | () -> Self | Deep copy (independent struct) |

User-defined methods are declared inside the `type` body. See
[Type Declarations](#4-3-type-declarations).

### 11.7 Tuple Properties

| Property          | Type   | Description             |
| ----------------- | ------ | ----------------------- |
| `t.length`        | i64    | Number of elements      |
| `t.0`, `t.1`, ... | varies | Element access by index |

---

## 12. Capabilities and IO

### 12.1 Capability Model

IO in Basalt is performed through capability objects passed to `main`:

```
fn main(stdout: Stdout, stdin: Stdin) {
    stdout.println("What is your name?")
    let name = stdin.read_line()
    stdout.println("Hello, \(name)!")
}
```

If `main` does not declare `stdout: Stdout`, the program cannot print. The
host controls what capabilities are available.

### 12.2 Stdout

| Method                 | Signature       | Description           |
| ---------------------- | --------------- | --------------------- |
| `stdout.println(text)` | (string) -> nil | Print with newline    |
| `stdout.print(text)`   | (string) -> nil | Print without newline |
| `stdout.flush()`       | () -> nil       | Flush output buffer   |

### 12.3 Stdin

| Method              | Signature    | Description                      |
| ------------------- | ------------ | -------------------------------- |
| `stdin.read_line()` | () -> string | Read one line (strips newline)   |
| `stdin.read_key()`  | () -> string | Read single character (raw mode) |

### 12.4 Host Objects

The embedding host can inject custom capability objects:

```rust
// Rust host code
struct GameApi;
impl HostObject for GameApi {
    fn call_method(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "get_time" => Ok(Value::float(0.0)),
            _ => Err(format!("unknown method: {name}"))
        }
    }
    fn type_name(&self) -> &str { "GameApi" }
}
vm.set_global("game", Value::host_object(GameApi));
```

---

## 13. Embedding API

### 13.1 Compilation

```rust
use basalt_core::Engine;

let engine = Engine::new();
let result = engine.run_source(source)?;
```

### 13.2 VM Interaction

```rust
let mut vm = basalt_vm::VM::new(program);

// Inject host globals
vm.set_global("game", Value::host_object(MyApi));

// Execute
let result = vm.run()?;
```

### 13.3 Value Types

The `Value` type is an untagged 8-byte slot. The host must use the
correct constructor for the intended type — the VM interprets the raw
bits according to the compiler's type information, not runtime tags.
Constructors:

```rust
Value::int(42)
Value::float(3.14)
Value::bool(true)
Value::string("hello".to_string())
Value::array(vec![Value::int(1), Value::int(2)])
Value::map(indexmap! { Value::string("a") => Value::int(1) })
Value::NIL
Value::host_object(my_obj)     // custom host object
```

### 13.4 Runtime Limits

| Limit             | Default | Description                                      |
| ----------------- | ------- | ------------------------------------------------ |
| Max call depth    | 512     | Prevents stack overflow from deep recursion      |
| Max registers     | 256K    | Prevents memory exhaustion from deep call stacks |
| Max instructions  | 100M    | Prevents infinite loops (execution fuel)         |
| Max string repeat | 16 MiB  | Prevents OOM from `string.repeat()`              |

---

## 14. Grammar Summary

```ebnf
program      = item* ;
item         = import_decl | function_def | type_def | let_decl ;

import_decl  = "import" STRING ("as" IDENT)? ;
function_def = "fn" IDENT "(" params? ")" ("->" type)? block ;
type_def     = "type" TYPE_IDENT (":" TYPE_IDENT)? "{" type_member* "}"
             | "type" TYPE_IDENT "=" type ;     (* type alias *)
let_decl     = "let" "mut"? IDENT (":" type)? "=" expr ;

type_member  = field_def | method_def | variant_def ;
field_def    = IDENT ":" type ;
method_def   = "fn" IDENT "(" params? ")" ("->" type)? block ;
variant_def  = TYPE_IDENT ("(" type ("," type)* ")")? ","? ;

params       = param ("," param)* ;
param        = IDENT ":" type ;

type         = base_type ("|" base_type)* ;    (* union is top-level *)
base_type    = qualified_type | PRIMITIVE | "Self" | array_type | map_type
             | tuple_type | optional_type | result_type | function_type ;
qualified_type = (IDENT ".")? TYPE_IDENT ;       (* e.g., shapes.Shape *)
array_type   = "[" type "]" ;
map_type     = "[" type ":" type "]" ;
tuple_type   = "(" type ("," type)+ ")" ;
optional_type = base_type "?" ;
result_type  = base_type "!" base_type ;
function_type = "fn" "(" (type ("," type)*)? ")" ("->" type)? ;

PRIMITIVE    = "i8" | "i16" | "i32" | "i64"
             | "u8" | "u16" | "u32" | "u64"
             | "f64" | "bool" | "string" | "nil" ;

block        = "{" statement* "}" ;
statement    = let_decl | assign_stmt | return_stmt | "break" | "continue" | expr ;
assign_stmt  = (IDENT | expr "." IDENT | expr "." TYPE_IDENT
             | expr "[" expr "]") "=" expr ;
return_stmt  = "return" expr? ;

literal      = INT | FLOAT | STRING | MULTILINE
             | "true" | "false" | "nil" ;

expr         = literal | IDENT | TYPE_IDENT
             | expr binop expr | unaryop expr
             | expr "(" args? ")"              (* call *)
             | expr "." IDENT "(" args? ")"    (* method call *)
             | expr "." IDENT                  (* field access *)
             | expr "." TYPE_IDENT             (* enum variant, static access *)
             | expr "." TYPE_IDENT "(" args? ")" (* variant construction, static call *)
             | expr "[" expr "]"               (* index *)
             | expr "?"                        (* try *)
             | expr "as" type                  (* type conversion — panics on failure *)
             | expr "as" "?" type              (* safe conversion — nil on failure *)
             | expr "is" type                  (* type test — union narrowing *)
             | expr "is" expr                  (* reference identity test *)
             | expr ".." expr                  (* range *)
             | "[" (expr ("," expr)* ","?)? "]"  (* array literal *)
             | "{" (expr ":" expr ("," expr ":" expr)* ","?)? "}" (* map literal *)
             | "(" expr ("," expr)+ ","? ")"   (* tuple literal *)
             | if_expr | match_expr | for_expr | while_expr
             | loop_expr | guard_expr | lambda | block
             | "!(" expr ")"                   (* error literal *)
             | TYPE_IDENT "{" (field_init ("," field_init)* ","?)? "}" (* struct *)
             ;

if_expr      = "if" expr block ("else" (block | if_expr))? ;
match_expr   = "match" expr "{" match_arm* "}" ;
match_arm    = pattern "=>" (expr | statement) ","? ;
for_expr     = "for" IDENT ("," IDENT)? "in" expr block ;
while_expr   = "while" expr block ;
loop_expr    = "loop" block ;
guard_expr   = "guard" ("let" IDENT "=")? expr "else" block ;
lambda       = "fn" "(" params? ")" ("->" type)? block ;

pattern      = "_" | IDENT | literal
             | TYPE_IDENT "." TYPE_IDENT
             | TYPE_IDENT "." TYPE_IDENT "(" IDENT ("," IDENT)* ")"
             | IDENT "." TYPE_IDENT "." TYPE_IDENT
             | IDENT "." TYPE_IDENT "." TYPE_IDENT "(" IDENT ("," IDENT)* ")"
             | "!" IDENT
             | "is" type                        (* union type narrowing *)
             | "is" TYPE_IDENT "." TYPE_IDENT ; (* enum variant test *)

field_init   = IDENT ":" expr ;
args         = expr ("," expr)* ","? ;

binop        = "||" | "&&" | "|" | "^" | "&"
             | "==" | "!="
             | "<" | "<=" | ">" | ">="
             | "<<" | ">>" | "+" | "-" | "*" | "/" | "%" | "**" ;
unaryop      = "-" | "!" ;

IDENT        = [a-z_][a-zA-Z0-9_]* ;
TYPE_IDENT   = [A-Z][a-zA-Z0-9_]* ;
INT          = [0-9]+ | "0x" [0-9a-fA-F]+ | "0b" [01]+ ;
FLOAT        = [0-9]+ "." [0-9]+ ([eE] [+-]? [0-9]+)? ;
STRING       = '"' (char | escape | interpolation)* '"' ;
MULTILINE    = ("\\" "\\" [^\n]* "\n")+ ;
```
