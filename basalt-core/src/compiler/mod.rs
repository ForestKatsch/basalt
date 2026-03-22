/// Basalt Bytecode Compiler - Generates bytecode from typed AST.
use crate::types::*;
use std::collections::HashMap;

/// Bytecode instructions. All register operands are u16.
/// Values are untagged 8-byte slots.
#[derive(Debug, Clone, Copy)]
pub enum Op {
    // Constants
    LoadInt(u16, i64),    // reg = i64 value
    LoadUInt(u16, u64),   // reg = u64 value
    LoadFloat(u16, f64),  // reg = f64 value
    LoadBool(u16, bool),  // reg = bool value
    LoadString(u16, u32), // reg = string constant index
    LoadNil(u16),         // reg = nil

    // Arithmetic (integer)
    AddInt(u16, u16, u16), // dst = a + b
    SubInt(u16, u16, u16), // dst = a - b
    MulInt(u16, u16, u16), // dst = a * b
    DivInt(u16, u16, u16), // dst = a / b
    ModInt(u16, u16, u16), // dst = a % b
    PowInt(u16, u16, u16), // dst = a ** b
    NegInt(u16, u16),      // dst = -src

    // Arithmetic (float)
    AddFloat(u16, u16, u16),
    SubFloat(u16, u16, u16),
    MulFloat(u16, u16, u16),
    DivFloat(u16, u16, u16),
    ModFloat(u16, u16, u16),
    PowFloat(u16, u16, u16),
    NegFloat(u16, u16),

    // String operations
    ConcatString(u16, u16, u16), // dst = a + b (string concat)

    // Comparison
    EqInt(u16, u16, u16),
    NeqInt(u16, u16, u16),
    LtInt(u16, u16, u16),
    LteInt(u16, u16, u16),
    GtInt(u16, u16, u16),
    GteInt(u16, u16, u16),

    EqFloat(u16, u16, u16),
    NeqFloat(u16, u16, u16),
    LtFloat(u16, u16, u16),
    LteFloat(u16, u16, u16),
    GtFloat(u16, u16, u16),
    GteFloat(u16, u16, u16),

    EqString(u16, u16, u16),
    NeqString(u16, u16, u16),
    LtString(u16, u16, u16),
    LteString(u16, u16, u16),
    GtString(u16, u16, u16),
    GteString(u16, u16, u16),

    EqBool(u16, u16, u16),
    NeqBool(u16, u16, u16),

    EqGeneric(u16, u16, u16), // deep structural equality
    NeqGeneric(u16, u16, u16),

    // Logical
    Not(u16, u16),      // dst = !src
    And(u16, u16, u16), // dst = a && b (but we use jumps for short-circuit)
    Or(u16, u16, u16),

    // Bitwise
    BitAnd(u16, u16, u16),
    BitOr(u16, u16, u16),
    BitXor(u16, u16, u16),
    BitNot(u16, u16),
    ShiftLeft(u16, u16, u16),
    ShiftRight(u16, u16, u16),

    // Type conversions
    IntToFloat(u16, u16),
    FloatToInt(u16, u16),
    FloatToIntSafe(u16, u16), // dst = float-to-int, nil on NaN/Infinity/overflow
    IntToString(u16, u16),
    FloatToString(u16, u16),
    BoolToString(u16, u16),
    StringToInt(u16, u16),     // panics on failure
    StringToFloat(u16, u16),   // panics on failure
    StringToIntSafe(u16, u16), // returns optional
    StringToFloatSafe(u16, u16),
    IntNarrow(u16, u16, IntType), // narrow i64 to smaller int type
    IntNarrowSafe(u16, u16, IntType),
    IntWiden(u16, u16), // widen narrow int to i64

    // Control flow
    Jump(i32),             // unconditional jump (relative offset)
    JumpIfTrue(u16, i32),  // jump if reg is true
    JumpIfFalse(u16, i32), // jump if reg is false
    JumpIfNil(u16, i32),   // jump if reg is nil
    JumpIfNotNil(u16, i32),
    JumpIfError(u16, i32), // jump if reg is an error value

    // Functions
    Call(u16, u16, u8), // dst = func(args...), func_reg, arg_count
    Return(u16),        // return value in reg
    ReturnNil,
    ReturnError(u16), // return error value

    // Collections
    MakeArray(u16, u16, u16), // dst, start_reg, count
    MakeMap(u16, u16, u16),   // dst, start_reg, entry_count (key/val pairs)
    MakeTuple(u16, u16, u16), // dst, start_reg, count

    // Struct
    MakeStruct(u16, u32, u8), // dst, type_id, field_count (fields in consecutive regs)
    GetField(u16, u16, u16),  // dst, obj, field_index
    SetField(u16, u16, u16),  // obj, field_index, value

    // Enum
    MakeEnum(u16, u32, u8, u8), // dst, type_id, variant_index, field_count
    GetEnumTag(u16, u16),       // dst = tag of enum value
    GetEnumField(u16, u16, u8), // dst, enum_val, field_index

    // Array/Map operations
    GetIndex(u16, u16, u16), // dst = arr[idx]
    SetIndex(u16, u16, u16), // arr[idx] = val
    ArrayLen(u16, u16),      // dst = len(arr)
    StringLen(u16, u16),     // dst = len(str)
    MapLen(u16, u16),

    // Method calls (built-in)
    CallMethod(u16, u16, u32, u8),     // dst, obj, method_id, arg_count
    CallCapability(u16, u16, u32, u8), // dst, cap, method_id, arg_count

    // String interpolation
    StringConcat(u16, u16, u16), // same as ConcatString

    // Error handling
    MakeError(u16, u16),   // dst = Error(value)
    UnwrapError(u16, u16), // dst = unwrap error value
    IsError(u16, u16),     // dst = is_error(value)
    IsNil(u16, u16),       // dst = is_nil(value)

    // Type testing
    IsType(u16, u16, u32),       // dst = value is type_id
    IsEnumVariant(u16, u16, u8), // dst = value is variant_index

    // Range
    MakeRange(u16, u16, u16), // dst = start..end

    // For loop support
    IterInit(u16, u16),             // iter = init_iterator(collection)
    IterNext(u16, u16, u16),        // (value, done) = next(iter), done_reg
    IterNextKV(u16, u16, u16, u16), // (key, value, done) = next(iter)

    // Generic display
    DisplayToString(u16, u16), // dst = display_as_string(src)

    // Panic
    Panic(u16), // panic with message in reg

    // Identity test
    IsIdentical(u16, u16, u16), // dst = a is b (reference identity)

    // Capture cells (for by-reference closure capture)
    MakeCell(u16, u16), // dst = new cell wrapping value in src
    CellGet(u16, u16),  // dst = read value from cell in src
    CellSet(u16, u16),  // cell[src1] = src2 (write value into cell)

    // Closures
    MakeClosure(u16, u16, u16), // dst, func_idx_reg, capture_count (captures in consecutive regs before dst)

    // Move
    Move(u16, u16), // dst = src

    // Nop
    Nop,

    // Halt
    Halt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

/// A compiled function.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub param_count: u8,
    pub register_count: u16,
    pub code: Vec<Op>,
    pub param_types: Vec<Type>,
    pub return_type: Type,
    /// Source line for each instruction (1-indexed, 0 = unknown).
    pub line_table: Vec<u32>,
}

/// Compiled program.
#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<CompiledFunction>,
    pub strings: Vec<String>,
    pub entry_point: usize, // index into functions for main()
    pub type_info: TypeInfo,
    pub method_names: Vec<String>,
    pub type_ids: Vec<String>, // type name -> type_id mapping
    pub globals: Vec<(String, Type)>,
    /// method_name -> [function_index] for O(1) method dispatch
    pub method_lookup: HashMap<String, Vec<usize>>,
}

pub fn compile(program: &TypedProgram) -> Result<Program, String> {
    codegen::compile_program(program)
}

mod capture;
mod codegen;
