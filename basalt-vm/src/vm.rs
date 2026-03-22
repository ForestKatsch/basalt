/// Basalt VM - Bytecode execution engine.
use crate::value::*;
use basalt_core::compiler::{IntType, Op, Program};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_CALL_DEPTH: usize = 256;
const MAX_INSTRUCTIONS: u64 = 100_000_000;
const MAX_STRING_REPEAT: usize = 16 * 1024 * 1024;

pub struct VM {
    program: Program,
    call_depth: usize,
    instruction_count: u64,
    pub captured_output: Vec<String>,
    stdin_lines: Vec<String>,
    stdin_pos: usize,
    /// Call stack for error reporting: (function_index, instruction_pointer)
    call_stack: Vec<(usize, usize)>,
    fs_root: Option<PathBuf>,
    env_args: Vec<String>,
}

/// Helper to extract a heap reference, returning a runtime error instead of panicking.
fn heap_ref<'a>(val: &'a Value, context: &str) -> Result<&'a HeapRef, String> {
    val.as_heap_ref().ok_or_else(|| {
        format!(
            "runtime error: expected heap value for {}, got {}",
            context,
            val.type_tag()
        )
    })
}

/// Helper to extract a heap object and match on it, producing a runtime error on mismatch.
macro_rules! with_heap {
    ($val:expr, $ctx:expr, $pat:pat => $body:expr) => {{
        let href = heap_ref($val, $ctx)?;
        let obj = href.borrow();
        match &*obj {
            $pat => $body,
            _ => return Err(format!("runtime error: unexpected heap type for {}", $ctx)),
        }
    }};
}

impl VM {
    pub fn new(program: Program) -> Self {
        VM {
            program,
            call_depth: 0,
            instruction_count: 0,
            captured_output: Vec::new(),
            stdin_lines: Vec::new(),
            stdin_pos: 0,
            call_stack: Vec::new(),
            fs_root: None,
            env_args: Vec::new(),
        }
    }

    pub fn set_stdin(&mut self, lines: Vec<String>) {
        self.stdin_lines = lines;
    }

    pub fn set_fs_root(&mut self, root: PathBuf) {
        self.fs_root = Some(root);
    }

    pub fn set_env_args(&mut self, args: Vec<String>) {
        self.env_args = args;
    }

    pub fn run(&mut self) -> Result<Value, String> {
        let entry = self.program.entry_point;
        let main_func = &self.program.functions[entry];

        // Construct capability args for main
        let mut args = Vec::new();
        for pt in &main_func.param_types {
            match pt {
                basalt_core::types::Type::Capability(name) => {
                    let cap = match name.as_str() {
                        "Stdout" => Value::capability("Stdout".into(), CapabilityData::Stdout),
                        "Stdin" => Value::capability("Stdin".into(), CapabilityData::Stdin),
                        "Fs" => Value::capability(
                            "Fs".into(),
                            CapabilityData::Fs {
                                root: self.fs_root.clone().unwrap_or_else(|| PathBuf::from(".")),
                            },
                        ),
                        "Env" => Value::capability(
                            "Env".into(),
                            CapabilityData::Env {
                                args: self.env_args.clone(),
                            },
                        ),
                        _ => Value::Nil,
                    };
                    args.push(cap);
                }
                _ => args.push(Value::Nil),
            }
        }

        self.call_function(entry, &args)
    }

    fn call_function(&mut self, func_idx: usize, args: &[Value]) -> Result<Value, String> {
        self.call_depth += 1;
        if self.call_depth > MAX_CALL_DEPTH {
            let trace = self.format_stack_trace();
            return Err(format!(
                "stack overflow: maximum call depth exceeded\nstack trace:\n{}",
                trace
            ));
        }

        // Check instruction budget at call boundaries (amortized)
        self.instruction_count += 1;
        if self.instruction_count > MAX_INSTRUCTIONS {
            return Err("execution limit exceeded".to_string());
        }

        let reg_count = self.program.functions[func_idx].register_count as usize;
        let reg_count = reg_count.max(args.len()).max(16);
        let mut registers = vec![Value::Nil; reg_count];

        for (i, arg) in args.iter().enumerate() {
            registers[i] = arg.clone();
        }

        self.call_stack.push((func_idx, 0));
        let result = self.execute(func_idx, &mut registers);
        self.call_depth -= 1;
        match result {
            Ok(v) => {
                self.call_stack.pop();
                Ok(v)
            }
            Err(msg) if !msg.contains("\nstack trace:\n") => {
                // Format trace while all frames (including this one) are still on the stack
                let trace = self.format_stack_trace();
                self.call_stack.pop();
                Err(format!("{}\nstack trace:\n{}", msg, trace))
            }
            err => {
                self.call_stack.pop();
                err
            }
        }
    }

    fn execute(&mut self, func_idx: usize, reg: &mut [Value]) -> Result<Value, String> {
        let code_len = self.program.functions[func_idx].code.len();
        let mut pc = 0;

        while pc < code_len {
            // Copy the instruction out (Op is Copy) to release borrow on self
            let op = self.program.functions[func_idx].code[pc];
            // Update call stack with current pc for stack traces
            if let Some(frame) = self.call_stack.last_mut() {
                frame.1 = pc;
            }
            pc += 1;

            match op {
                // === Constants ===
                Op::LoadInt(d, n) => {
                    reg[d as usize] = Value::int(n);
                }
                Op::LoadFloat(d, f) => {
                    reg[d as usize] = Value::float(f);
                }
                Op::LoadBool(d, b) => {
                    reg[d as usize] = Value::bool(b);
                }
                Op::LoadString(d, idx) => {
                    let s = self.program.strings[idx as usize].clone();
                    reg[d as usize] = Value::string(s);
                }
                Op::LoadNil(d) => {
                    reg[d as usize] = Value::Nil;
                }
                Op::Move(d, s) => {
                    let v = reg[s as usize].clone();
                    reg[d as usize] = v;
                }

                // === Integer Arithmetic ===
                Op::AddInt(d, a, b) => {
                    let va = reg[a as usize].as_int();
                    let vb = reg[b as usize].as_int();
                    reg[d as usize] = match va.checked_add(vb) {
                        Some(r) => Value::int(r),
                        None => {
                            return Err(format!(
                                "integer overflow: {} + {} exceeds i64 range",
                                va, vb
                            ))
                        }
                    };
                }
                Op::SubInt(d, a, b) => {
                    let va = reg[a as usize].as_int();
                    let vb = reg[b as usize].as_int();
                    reg[d as usize] = match va.checked_sub(vb) {
                        Some(r) => Value::int(r),
                        None => {
                            return Err(format!(
                                "integer overflow: {} - {} exceeds i64 range",
                                va, vb
                            ))
                        }
                    };
                }
                Op::MulInt(d, a, b) => {
                    let va = reg[a as usize].as_int();
                    let vb = reg[b as usize].as_int();
                    reg[d as usize] = match va.checked_mul(vb) {
                        Some(r) => Value::int(r),
                        None => {
                            return Err(format!(
                                "integer overflow: {} * {} exceeds i64 range",
                                va, vb
                            ))
                        }
                    };
                }
                Op::DivInt(d, a, b) => {
                    let vb = reg[b as usize].as_int();
                    if vb == 0 {
                        return Err("division by zero".to_string());
                    }
                    reg[d as usize] = Value::int(reg[a as usize].as_int() / vb);
                }
                Op::ModInt(d, a, b) => {
                    let vb = reg[b as usize].as_int();
                    if vb == 0 {
                        return Err("modulo by zero".to_string());
                    }
                    reg[d as usize] = Value::int(reg[a as usize].as_int() % vb);
                }
                Op::PowInt(d, a, b) => {
                    let base = reg[a as usize].as_int();
                    let exp = reg[b as usize].as_int();
                    if exp < 0 {
                        return Err("negative exponent for integer power".to_string());
                    }
                    reg[d as usize] = match checked_pow_i64(base, exp as u64) {
                        Some(r) => Value::int(r),
                        None => {
                            return Err(format!(
                                "integer overflow: {} ** {} exceeds i64 range",
                                base, exp
                            ))
                        }
                    };
                }
                Op::NegInt(d, s) => {
                    let v = reg[s as usize].as_int();
                    reg[d as usize] = match v.checked_neg() {
                        Some(r) => Value::int(r),
                        None => return Err(format!("integer overflow: -{} exceeds i64 range", v)),
                    };
                }

                // === Float Arithmetic ===
                Op::AddFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::float(reg[a as usize].as_float() + reg[b as usize].as_float());
                }
                Op::SubFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::float(reg[a as usize].as_float() - reg[b as usize].as_float());
                }
                Op::MulFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::float(reg[a as usize].as_float() * reg[b as usize].as_float());
                }
                Op::DivFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::float(reg[a as usize].as_float() / reg[b as usize].as_float());
                }
                Op::ModFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::float(reg[a as usize].as_float() % reg[b as usize].as_float());
                }
                Op::PowFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::float(reg[a as usize].as_float().powf(reg[b as usize].as_float()));
                }
                Op::NegFloat(d, s) => {
                    reg[d as usize] = Value::float(-reg[s as usize].as_float());
                }

                // === String ===
                Op::ConcatString(d, a, b) | Op::StringConcat(d, a, b) => {
                    let sa = self.val_to_string(&reg[a as usize]);
                    let sb = self.val_to_string(&reg[b as usize]);
                    reg[d as usize] = Value::string(sa + &sb);
                }

                // === Integer Comparisons ===
                Op::EqInt(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_int() == reg[b as usize].as_int());
                }
                Op::NeqInt(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_int() != reg[b as usize].as_int());
                }
                Op::LtInt(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_int() < reg[b as usize].as_int());
                }
                Op::LteInt(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_int() <= reg[b as usize].as_int());
                }
                Op::GtInt(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_int() > reg[b as usize].as_int());
                }
                Op::GteInt(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_int() >= reg[b as usize].as_int());
                }

                // === Float Comparisons ===
                Op::EqFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_float() == reg[b as usize].as_float());
                }
                Op::NeqFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_float() != reg[b as usize].as_float());
                }
                Op::LtFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_float() < reg[b as usize].as_float());
                }
                Op::LteFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_float() <= reg[b as usize].as_float());
                }
                Op::GtFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_float() > reg[b as usize].as_float());
                }
                Op::GteFloat(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_float() >= reg[b as usize].as_float());
                }

                // === String Comparisons ===
                Op::EqString(d, a, b) => {
                    reg[d as usize] = Value::bool(
                        self.val_to_string(&reg[a as usize])
                            == self.val_to_string(&reg[b as usize]),
                    );
                }
                Op::NeqString(d, a, b) => {
                    reg[d as usize] = Value::bool(
                        self.val_to_string(&reg[a as usize])
                            != self.val_to_string(&reg[b as usize]),
                    );
                }
                Op::LtString(d, a, b) => {
                    reg[d as usize] = Value::bool(
                        self.val_to_string(&reg[a as usize]) < self.val_to_string(&reg[b as usize]),
                    );
                }
                Op::LteString(d, a, b) => {
                    reg[d as usize] = Value::bool(
                        self.val_to_string(&reg[a as usize])
                            <= self.val_to_string(&reg[b as usize]),
                    );
                }
                Op::GtString(d, a, b) => {
                    reg[d as usize] = Value::bool(
                        self.val_to_string(&reg[a as usize]) > self.val_to_string(&reg[b as usize]),
                    );
                }
                Op::GteString(d, a, b) => {
                    reg[d as usize] = Value::bool(
                        self.val_to_string(&reg[a as usize])
                            >= self.val_to_string(&reg[b as usize]),
                    );
                }

                // === Bool Comparisons ===
                Op::EqBool(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_bool() == reg[b as usize].as_bool());
                }
                Op::NeqBool(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_bool() != reg[b as usize].as_bool());
                }

                // === Generic Equality ===
                Op::EqGeneric(d, a, b) => {
                    reg[d as usize] = Value::bool(reg[a as usize].deep_eq(&reg[b as usize]));
                }
                Op::NeqGeneric(d, a, b) => {
                    reg[d as usize] = Value::bool(!reg[a as usize].deep_eq(&reg[b as usize]));
                }

                // === Logical ===
                Op::Not(d, s) => {
                    reg[d as usize] = Value::bool(!reg[s as usize].as_bool());
                }
                Op::And(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_bool() && reg[b as usize].as_bool());
                }
                Op::Or(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(reg[a as usize].as_bool() || reg[b as usize].as_bool());
                }

                // === Bitwise ===
                Op::BitAnd(d, a, b) => {
                    reg[d as usize] =
                        Value::int(reg[a as usize].as_int() & reg[b as usize].as_int());
                }
                Op::BitOr(d, a, b) => {
                    reg[d as usize] =
                        Value::int(reg[a as usize].as_int() | reg[b as usize].as_int());
                }
                Op::BitXor(d, a, b) => {
                    reg[d as usize] =
                        Value::int(reg[a as usize].as_int() ^ reg[b as usize].as_int());
                }
                Op::BitNot(d, s) => {
                    reg[d as usize] = Value::int(!reg[s as usize].as_int());
                }
                Op::ShiftLeft(d, a, b) => {
                    let shift = reg[b as usize].as_int();
                    if !(0..=63).contains(&shift) {
                        return Err(format!("shift amount {} out of range", shift));
                    }
                    reg[d as usize] = Value::int(reg[a as usize].as_int() << shift);
                }
                Op::ShiftRight(d, a, b) => {
                    let shift = reg[b as usize].as_int();
                    if !(0..=63).contains(&shift) {
                        return Err(format!("shift amount {} out of range", shift));
                    }
                    reg[d as usize] = Value::int(reg[a as usize].as_int() >> shift);
                }

                // === Type Conversions ===
                Op::IntToFloat(d, s) => {
                    reg[d as usize] = Value::float(reg[s as usize].as_int() as f64);
                }
                Op::FloatToInt(d, s) => {
                    let f = reg[s as usize].as_float();
                    if f.is_nan() || f.is_infinite() {
                        return Err(format!("cannot convert {} to integer", f));
                    }
                    reg[d as usize] = Value::int(f as i64);
                }
                Op::FloatToIntSafe(d, s) => {
                    let f = reg[s as usize].as_float();
                    if f.is_nan() || f.is_infinite() || f > i64::MAX as f64 || f < i64::MIN as f64 {
                        reg[d as usize] = Value::Nil;
                    } else {
                        reg[d as usize] = Value::int(f as i64);
                    }
                }
                Op::IntToString(d, s) => {
                    reg[d as usize] = Value::string(reg[s as usize].as_int().to_string());
                }
                Op::FloatToString(d, s) => {
                    reg[d as usize] = Value::string(format_float(reg[s as usize].as_float()));
                }
                Op::BoolToString(d, s) => {
                    reg[d as usize] = Value::string(
                        if reg[s as usize].as_bool() {
                            "true"
                        } else {
                            "false"
                        }
                        .to_string(),
                    );
                }
                Op::StringToInt(d, s) => {
                    let sv = self.val_to_string(&reg[s as usize]);
                    reg[d as usize] = Value::int(
                        sv.trim()
                            .parse::<i64>()
                            .map_err(|_| format!("cannot convert '{}' to i64", sv))?,
                    );
                }
                Op::StringToFloat(d, s) => {
                    let sv = self.val_to_string(&reg[s as usize]);
                    reg[d as usize] = Value::float(
                        sv.trim()
                            .parse::<f64>()
                            .map_err(|_| format!("cannot convert '{}' to f64", sv))?,
                    );
                }
                Op::StringToIntSafe(d, s) => {
                    let sv = self.val_to_string(&reg[s as usize]);
                    reg[d as usize] = match sv.trim().parse::<i64>() {
                        Ok(n) => Value::int(n),
                        Err(_) => Value::Nil,
                    };
                }
                Op::StringToFloatSafe(d, s) => {
                    let sv = self.val_to_string(&reg[s as usize]);
                    reg[d as usize] = match sv.trim().parse::<f64>() {
                        Ok(f) => Value::float(f),
                        Err(_) => Value::Nil,
                    };
                }
                Op::IntNarrow(d, s, it) => {
                    reg[d as usize] = Value::int(narrow_int(reg[s as usize].as_int(), it)?);
                }
                Op::IntNarrowSafe(d, s, it) => {
                    reg[d as usize] = match narrow_int(reg[s as usize].as_int(), it) {
                        Ok(v) => Value::int(v),
                        Err(_) => Value::Nil,
                    };
                }
                Op::IntWiden(d, s) => {
                    reg[d as usize] = reg[s as usize].clone();
                }

                // === Control Flow ===
                Op::Jump(off) => {
                    if off <= 0 {
                        // Backward jump (loop) — check instruction budget
                        self.instruction_count += 1;
                        if self.instruction_count > MAX_INSTRUCTIONS {
                            return Err("execution limit exceeded".to_string());
                        }
                    }
                    pc = (pc as i32 + off - 1) as usize;
                }
                Op::JumpIfTrue(r, off) => {
                    if reg[r as usize].as_bool() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfFalse(r, off) => {
                    if !reg[r as usize].as_bool() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfNil(r, off) => {
                    if reg[r as usize].is_nil() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfNotNil(r, off) => {
                    if !reg[r as usize].is_nil() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfError(r, off) => {
                    if reg[r as usize].is_error() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }

                // === Function Calls ===
                Op::Call(d, fr, ac) => {
                    let func_val = &reg[fr as usize];
                    let mut args = Vec::with_capacity(ac as usize);
                    let start = fr as usize + 1;
                    for i in 0..ac as usize {
                        args.push(reg[start + i].clone());
                    }

                    // Check if calling a closure or a plain function index
                    if let Some(href) = func_val.as_heap_ref() {
                        let obj = href.borrow();
                        if let HeapObject::Closure(closure) = &*obj {
                            let func_idx = closure.func_idx;
                            let mut full_args = closure.captures.clone();
                            full_args.extend(args);
                            drop(obj);
                            reg[d as usize] = self.call_function(func_idx, &full_args)?;
                        } else {
                            return Err("cannot call non-function value".to_string());
                        }
                    } else {
                        let func_call_idx = func_val.as_int() as usize;
                        if func_call_idx < self.program.functions.len() {
                            reg[d as usize] = self.call_function(func_call_idx, &args)?;
                        } else {
                            reg[d as usize] = Value::Nil;
                        }
                    }
                }
                Op::Return(r) => {
                    return Ok(reg[r as usize].clone());
                }
                Op::ReturnNil => {
                    return Ok(Value::Nil);
                }
                Op::ReturnError(r) => {
                    return Ok(reg[r as usize].clone());
                }

                // === Collections ===
                Op::MakeArray(d, start, count) => {
                    let mut vals = Vec::with_capacity(count as usize);
                    for i in 0..count as usize {
                        vals.push(reg[start as usize + i].clone());
                    }
                    reg[d as usize] = Value::array(vals);
                }
                Op::MakeMap(d, start, count) => {
                    let mut map = IndexMap::new();
                    for i in 0..count as usize {
                        let ki = start as usize + (i * 2);
                        let vi = ki + 1;
                        let key = val_to_map_key(&reg[ki])?;
                        map.insert(key, reg[vi].clone());
                    }
                    reg[d as usize] = Value::map(map);
                }
                Op::MakeTuple(d, start, count) => {
                    let mut vals = Vec::with_capacity(count as usize);
                    for i in 0..count as usize {
                        vals.push(reg[start as usize + i].clone());
                    }
                    reg[d as usize] = Value::tuple(vals);
                }

                // === Struct ===
                Op::MakeStruct(d, tid, fc) => {
                    let tn = self.program.type_ids[tid as usize].clone();
                    let start = (d as usize).saturating_sub(fc as usize);
                    let mut fields = Vec::with_capacity(fc as usize);
                    for i in 0..fc as usize {
                        fields.push(reg[start + i].clone());
                    }
                    reg[d as usize] = Value::new_struct(tn, fields);
                }
                Op::GetField(d, o, fi) => {
                    let href = heap_ref(&reg[o as usize], "field access")?.clone();
                    let obj = href.borrow();
                    reg[d as usize] = match &*obj {
                        HeapObject::Struct(s) => {
                            s.fields.get(fi as usize).cloned().ok_or_else(|| {
                                format!(
                                    "field index {} out of bounds on struct '{}'",
                                    fi, s.type_name
                                )
                            })?
                        }
                        HeapObject::Tuple(vals) => {
                            vals.get(fi as usize).cloned().ok_or_else(|| {
                                format!("tuple index {} out of bounds (length {})", fi, vals.len())
                            })?
                        }
                        _ => return Err("field access on non-struct/tuple".to_string()),
                    };
                }
                Op::SetField(o, fi, v) => {
                    let value = reg[v as usize].clone();
                    let href = heap_ref(&reg[o as usize], "field assignment")?.clone();
                    let mut obj = href.borrow_mut();
                    match &mut *obj {
                        HeapObject::Struct(s) => {
                            if (fi as usize) < s.fields.len() {
                                s.fields[fi as usize] = value;
                            } else {
                                return Err(format!("field index {} out of bounds", fi));
                            }
                        }
                        _ => return Err("field assignment on non-struct".to_string()),
                    }
                }

                // === Enum ===
                Op::MakeEnum(d, tid, vi, fc) => {
                    let tn = self.program.type_ids[tid as usize].clone();
                    let start = (d as usize).saturating_sub(fc as usize);
                    let mut fields = Vec::with_capacity(fc as usize);
                    for i in 0..fc as usize {
                        fields.push(reg[start + i].clone());
                    }
                    reg[d as usize] = Value::new_enum(tn, vi, fields);
                }
                Op::GetEnumTag(d, s) => {
                    reg[d as usize] = with_heap!(&reg[s as usize], "enum tag",
                        HeapObject::Enum(e) => Value::int(e.variant_index as i64));
                }
                Op::GetEnumField(d, s, fi) => {
                    let href = heap_ref(&reg[s as usize], "enum field")?.clone();
                    let obj = href.borrow();
                    reg[d as usize] = match &*obj {
                        HeapObject::Enum(e) => e
                            .fields
                            .get(fi as usize)
                            .cloned()
                            .ok_or_else(|| format!("enum field index {} out of bounds", fi))?,
                        _ => return Err("enum field access on non-enum".to_string()),
                    };
                }

                // === Index ===
                Op::GetIndex(d, o, i) => {
                    reg[d as usize] = self.get_index(&reg[o as usize], &reg[i as usize])?;
                }
                Op::SetIndex(o, i, v) => {
                    self.set_index(&reg[o as usize], &reg[i as usize], reg[v as usize].clone())?;
                }
                Op::ArrayLen(d, s) => {
                    reg[d as usize] = with_heap!(&reg[s as usize], "array length",
                        HeapObject::Array(v) => Value::int(v.len() as i64));
                }
                Op::StringLen(d, s) => {
                    let sv = self.val_to_string(&reg[s as usize]);
                    reg[d as usize] = Value::int(sv.chars().count() as i64);
                }
                Op::MapLen(d, s) => {
                    reg[d as usize] = with_heap!(&reg[s as usize], "map length",
                        HeapObject::Map(m) => Value::int(m.len() as i64));
                }

                // === Method Calls ===
                Op::CallMethod(d, o, mid, ac) => {
                    let method = self.program.method_names[mid as usize].clone();
                    let mut args = Vec::with_capacity(ac as usize);
                    let start = o as usize + 1;
                    for i in 0..ac as usize {
                        args.push(reg[start + i].clone());
                    }
                    let obj = reg[o as usize].clone();
                    reg[d as usize] = self.call_method(&obj, &method, &args)?;
                }
                Op::CallCapability(d, c, mid, ac) => {
                    let method = self.program.method_names[mid as usize].clone();
                    let mut args = Vec::with_capacity(ac as usize);
                    let start = c as usize + 1;
                    for i in 0..ac as usize {
                        args.push(reg[start + i].clone());
                    }
                    let cap_val = reg[c as usize].clone();
                    reg[d as usize] = self.call_capability_on(&cap_val, &method, &args)?;
                }

                // === Error Handling ===
                Op::MakeError(d, s) => {
                    let v = reg[s as usize].clone();
                    reg[d as usize] = Value::error(v);
                }
                Op::UnwrapError(d, s) => {
                    let href = heap_ref(&reg[s as usize], "error unwrap")?.clone();
                    let obj = href.borrow();
                    reg[d as usize] = match &*obj {
                        HeapObject::Error(inner) => inner.as_ref().clone(),
                        _ => return Err("unwrap on non-error value".to_string()),
                    };
                }
                Op::IsError(d, s) => {
                    reg[d as usize] = Value::bool(reg[s as usize].is_error());
                }
                Op::IsNil(d, s) => {
                    reg[d as usize] = Value::bool(reg[s as usize].is_nil());
                }

                // === Type Testing ===
                Op::IsType(d, s, tid) => {
                    let type_name = self.program.type_ids[tid as usize].clone();
                    reg[d as usize] = Value::bool(value_is_type(&reg[s as usize], &type_name));
                }
                Op::IsEnumVariant(d, s, vi) => {
                    let is_match = if let Some(href) = reg[s as usize].as_heap_ref() {
                        let obj = href.borrow();
                        match &*obj {
                            HeapObject::Enum(e) => e.variant_index == vi,
                            _ => false,
                        }
                    } else {
                        false
                    };
                    reg[d as usize] = Value::bool(is_match);
                }
                // === Identity ===
                Op::IsIdentical(d, a, b) => {
                    reg[d as usize] =
                        Value::bool(values_identical(&reg[a as usize], &reg[b as usize]));
                }

                // === Range ===
                Op::MakeRange(d, s, e) => {
                    reg[d as usize] =
                        Value::range(reg[s as usize].as_int(), reg[e as usize].as_int());
                }

                // === Iterators ===
                Op::IterInit(d, s) => {
                    let iter = self.create_iterator(&reg[s as usize])?;
                    reg[d as usize] =
                        Value::Heap(Arc::new(RefCell::new(HeapObject::Iterator(iter))));
                }
                Op::IterNext(vd, dd, ir) => {
                    let href = heap_ref(&reg[ir as usize], "iterator next")?.clone();
                    let mut obj = href.borrow_mut();
                    let iter = match &mut *obj {
                        HeapObject::Iterator(iter) => iter,
                        _ => return Err("IterNext on non-iterator".to_string()),
                    };
                    match iter {
                        IterState::Array { source, index } => {
                            let src = source.borrow();
                            let values = match &*src {
                                HeapObject::Array(v) => v,
                                _ => return Err("iterator source is not an array".to_string()),
                            };
                            if *index < values.len() {
                                reg[vd as usize] = values[*index].clone();
                                reg[dd as usize] = Value::bool(false);
                                *index += 1;
                            } else {
                                reg[dd as usize] = Value::bool(true);
                            }
                        }
                        IterState::String { chars, index } => {
                            if *index < chars.len() {
                                reg[vd as usize] = Value::string(chars[*index].clone());
                                reg[dd as usize] = Value::bool(false);
                                *index += 1;
                            } else {
                                reg[dd as usize] = Value::bool(true);
                            }
                        }
                        IterState::Range { current, end } => {
                            if *current < *end {
                                reg[vd as usize] = Value::int(*current);
                                reg[dd as usize] = Value::bool(false);
                                *current += 1;
                            } else {
                                reg[dd as usize] = Value::bool(true);
                            }
                        }
                        IterState::Map {
                            keys,
                            source: _,
                            index,
                        } => {
                            if *index < keys.len() {
                                reg[vd as usize] = map_key_to_value(&keys[*index]);
                                reg[dd as usize] = Value::bool(false);
                                *index += 1;
                            } else {
                                reg[dd as usize] = Value::bool(true);
                            }
                        }
                    }
                }
                Op::IterNextKV(kd, vd, dd, ir) => {
                    let href = heap_ref(&reg[ir as usize], "iterator next_kv")?.clone();
                    let mut obj = href.borrow_mut();
                    let iter = match &mut *obj {
                        HeapObject::Iterator(iter) => iter,
                        _ => return Err("IterNextKV on non-iterator".to_string()),
                    };
                    match iter {
                        IterState::Array { source, index } => {
                            let src = source.borrow();
                            let values = match &*src {
                                HeapObject::Array(v) => v,
                                _ => return Err("iterator source is not an array".to_string()),
                            };
                            if *index < values.len() {
                                reg[kd as usize] = values[*index].clone();
                                reg[vd as usize] = Value::int(*index as i64);
                                reg[dd as usize] = Value::bool(false);
                                *index += 1;
                            } else {
                                reg[dd as usize] = Value::bool(true);
                            }
                        }
                        IterState::Map {
                            keys,
                            source,
                            index,
                        } => {
                            let src = source.borrow();
                            let map = match &*src {
                                HeapObject::Map(m) => m,
                                _ => return Err("iterator source is not a map".to_string()),
                            };
                            if *index < keys.len() {
                                reg[kd as usize] = map_key_to_value(&keys[*index]);
                                reg[vd as usize] =
                                    map.get(&keys[*index]).cloned().unwrap_or(Value::Nil);
                                reg[dd as usize] = Value::bool(false);
                                *index += 1;
                            } else {
                                reg[dd as usize] = Value::bool(true);
                            }
                        }
                        _ => return Err("IterNextKV requires array or map iterator".to_string()),
                    }
                }

                // === Capture Cells ===
                Op::MakeCell(d, s) => {
                    let val = reg[s as usize].clone();
                    reg[d as usize] = Value::Heap(Arc::new(RefCell::new(HeapObject::CaptureCell(
                        Box::new(val),
                    ))));
                }
                Op::CellGet(d, s) => {
                    let href = heap_ref(&reg[s as usize], "cell read")?.clone();
                    let obj = href.borrow();
                    reg[d as usize] = match &*obj {
                        HeapObject::CaptureCell(val) => val.as_ref().clone(),
                        _ => return Err("CellGet on non-cell".to_string()),
                    };
                }
                Op::CellSet(cell, val) => {
                    let new_val = reg[val as usize].clone();
                    let href = heap_ref(&reg[cell as usize], "cell write")?.clone();
                    let mut obj = href.borrow_mut();
                    match &mut *obj {
                        HeapObject::CaptureCell(inner) => **inner = new_val,
                        _ => return Err("CellSet on non-cell".to_string()),
                    }
                }

                // === Closures ===
                Op::MakeClosure(d, func_reg, capture_count) => {
                    let func_idx = reg[func_reg as usize].as_int() as usize;
                    let cap_start = func_reg as usize + 1;
                    let mut captures = Vec::with_capacity(capture_count as usize);
                    for i in 0..capture_count as usize {
                        captures.push(reg[cap_start + i].clone());
                    }
                    reg[d as usize] = Value::closure(func_idx, captures);
                }

                // === Display ===
                Op::DisplayToString(d, s) => {
                    reg[d as usize] = Value::string(reg[s as usize].display_as_string());
                }

                // === Misc ===
                Op::Panic(r) => {
                    return Err(format!("panic: {}", self.val_to_string(&reg[r as usize])));
                }
                Op::Nop => {}
                Op::Halt => {
                    return Ok(Value::Nil);
                }
            }
        }

        Ok(Value::Nil)
    }

    fn format_stack_trace(&self) -> String {
        let frames: Vec<(String, u32)> = self
            .call_stack
            .iter()
            .rev()
            .map(|&(fi, ip)| {
                let func = &self.program.functions[fi];
                let line = func.line_table.get(ip).copied().unwrap_or(0);
                (func.name.clone(), line)
            })
            .collect();

        let mut trace = String::new();

        let format_frame = |name: &str, line: u32| -> String {
            if line > 0 {
                format!("  at {} (line {})\n", name, line)
            } else {
                format!("  at {}\n", name)
            }
        };

        if frames.len() <= 16 {
            for (name, line) in &frames {
                trace.push_str(&format_frame(name, *line));
            }
        } else {
            // Show first 8 frames (most recent)
            for (name, line) in frames.iter().take(8) {
                trace.push_str(&format_frame(name, *line));
            }

            let omitted = frames.len() - 11; // 8 top + 3 bottom
            trace.push_str(&format!("  ... {} more frames\n", omitted));

            // Show last 3 frames (outermost callers, usually main)
            for (name, line) in frames.iter().skip(frames.len() - 3) {
                trace.push_str(&format_frame(name, *line));
            }
        }

        trace
    }

    fn val_to_string(&self, val: &Value) -> String {
        match val {
            Value::Heap(href) => {
                let obj = href.borrow();
                if let HeapObject::String(s) = &*obj {
                    return s.clone();
                }
                drop(obj);
                val.display_as_string()
            }
            _ => val.display_as_string(),
        }
    }

    fn get_index(&self, obj: &Value, idx: &Value) -> Result<Value, String> {
        let href = heap_ref(obj, "index access")?;
        let o = href.borrow();
        match &*o {
            HeapObject::Array(vals) => {
                let mut i = idx.as_int();
                if i < 0 {
                    i += vals.len() as i64;
                }
                vals.get(i as usize).cloned().ok_or_else(|| {
                    format!(
                        "array index {} out of bounds (length {})",
                        idx.as_int(),
                        vals.len()
                    )
                })
            }
            HeapObject::Map(entries) => {
                let key = val_to_map_key(idx)?;
                entries
                    .get(&key)
                    .cloned()
                    .ok_or("key not found in map".to_string())
            }
            _ => Err(format!("cannot index into {}", obj.display_as_string())),
        }
    }

    fn set_index(&self, obj: &Value, idx: &Value, val: Value) -> Result<(), String> {
        let href = heap_ref(obj, "index assignment")?;
        let mut o = href.borrow_mut();
        match &mut *o {
            HeapObject::Array(vals) => {
                let mut i = idx.as_int();
                if i < 0 {
                    i += vals.len() as i64;
                }
                if i < 0 || i as usize >= vals.len() {
                    return Err(format!(
                        "array index {} out of bounds (length {})",
                        idx.as_int(),
                        vals.len()
                    ));
                }
                vals[i as usize] = val;
                Ok(())
            }
            HeapObject::Map(entries) => {
                let key = val_to_map_key(idx)?;
                entries.insert(key, val);
                Ok(())
            }
            _ => Err(format!("cannot index into {}", obj.display_as_string())),
        }
    }

    fn create_iterator(&self, val: &Value) -> Result<IterState, String> {
        let href = heap_ref(val, "iteration")?;
        let obj = href.borrow();
        match &*obj {
            HeapObject::Array(_) => Ok(IterState::Array {
                source: href.clone(),
                index: 0,
            }),
            HeapObject::Map(m) => Ok(IterState::Map {
                keys: m.keys().cloned().collect(),
                source: href.clone(),
                index: 0,
            }),
            HeapObject::String(s) => Ok(IterState::String {
                chars: s.chars().map(|c| c.to_string()).collect(),
                index: 0,
            }),
            &HeapObject::Range(s, e) => Ok(IterState::Range { current: s, end: e }),
            _ => Err(format!("cannot iterate over {}", val.display_as_string())),
        }
    }

    fn call_method(&mut self, obj: &Value, method: &str, args: &[Value]) -> Result<Value, String> {
        // Module.function calls
        if method.contains('.') {
            let parts: Vec<&str> = method.splitn(2, '.').collect();
            if parts[0] == "math" {
                return self.call_math(parts[1], args);
            }
            // Look up compiled function by name via method_lookup
            if let Some(indices) = self.program.method_lookup.get(parts[1]) {
                if let Some(&idx) = indices.first() {
                    return self.call_function(idx, args);
                }
            }
            return Err(format!("unknown function '{}'", method));
        }

        if let Some(href) = obj.as_heap_ref() {
            let obj_borrow = href.borrow();
            match &*obj_borrow {
                HeapObject::String(s) => {
                    let s = s.clone();
                    drop(obj_borrow);
                    return self.call_string_method(&s, method, args);
                }
                HeapObject::Array(_) => {
                    drop(obj_borrow);
                    return self.call_array_method(href, method, args);
                }
                HeapObject::Map(_) => {
                    drop(obj_borrow);
                    return self.call_map_method(href, method, args);
                }
                HeapObject::Struct(s) => {
                    if method == "clone" {
                        return Ok(Value::new_struct(s.type_name.clone(), s.fields.clone()));
                    }
                    let type_name = s.type_name.clone();
                    drop(obj_borrow);
                    // Look for user methods via method_lookup
                    if let Some(indices) = self.program.method_lookup.get(method) {
                        if let Some(&idx) = indices.first() {
                            let mut call_args = vec![obj.clone()];
                            call_args.extend_from_slice(args);
                            return self.call_function(idx, &call_args);
                        }
                    }
                    return Err(format!("unknown method '{}' on '{}'", method, type_name));
                }
                HeapObject::Enum(e) => {
                    let type_name = e.type_name.clone();
                    drop(obj_borrow);
                    if let Some(indices) = self.program.method_lookup.get(method) {
                        if let Some(&idx) = indices.first() {
                            let mut call_args = vec![obj.clone()];
                            call_args.extend_from_slice(args);
                            return self.call_function(idx, &call_args);
                        }
                    }
                    return Err(format!("unknown method '{}' on '{}'", method, type_name));
                }
                _ => {
                    return Err(format!(
                        "cannot call method '{}' on {}",
                        method,
                        obj.display_as_string()
                    ));
                }
            }
        }
        Err(format!(
            "cannot call method '{}' on non-object value",
            method
        ))
    }

    fn call_capability_on(
        &mut self,
        cap: &Value,
        method: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        let href = cap
            .as_heap_ref()
            .ok_or_else(|| format!("expected capability object, got {}", cap.type_tag()))?;
        let obj = href.borrow();
        let cap_obj = match &*obj {
            HeapObject::Capability(c) => c,
            _ => return Err("expected capability object".to_string()),
        };
        let cap_name = cap_obj.name.clone();
        let data = cap_obj.data.clone();
        drop(obj); // Release borrow before calling methods that may need self

        match (cap_name.as_str(), method) {
            // === Stdout ===
            ("Stdout", "println") => {
                let text = if !args.is_empty() {
                    self.val_to_string(&args[0])
                } else {
                    String::new()
                };
                self.captured_output.push(text.clone());
                println!("{}", text);
                Ok(Value::Nil)
            }
            ("Stdout", "print") => {
                let text = if !args.is_empty() {
                    self.val_to_string(&args[0])
                } else {
                    String::new()
                };
                self.captured_output.push(text.clone());
                print!("{}", text);
                Ok(Value::Nil)
            }
            ("Stdout", "flush") => {
                use std::io::Write;
                std::io::stdout().flush().ok();
                Ok(Value::Nil)
            }

            // === Stdin ===
            ("Stdin", "read_line") => {
                if self.stdin_pos < self.stdin_lines.len() {
                    let line = self.stdin_lines[self.stdin_pos].clone();
                    self.stdin_pos += 1;
                    Ok(Value::string(line))
                } else {
                    let mut input = String::new();
                    std::io::stdin()
                        .read_line(&mut input)
                        .map_err(|e| format!("stdin: {}", e))?;
                    Ok(Value::string(input.trim_end_matches('\n').to_string()))
                }
            }
            ("Stdin", "read_key") => {
                if self.stdin_pos < self.stdin_lines.len() {
                    let line = &self.stdin_lines[self.stdin_pos];
                    let ch = line.chars().next().unwrap_or('\0').to_string();
                    self.stdin_pos += 1;
                    Ok(Value::string(ch))
                } else {
                    let mut buf = [0u8; 4];
                    let n = std::io::stdin()
                        .read(&mut buf)
                        .map_err(|e| format!("stdin: {}", e))?;
                    let s = std::str::from_utf8(&buf[..n]).unwrap_or("").to_string();
                    Ok(Value::string(s))
                }
            }

            // === Fs (file system, sandboxed) ===
            ("Fs", "read_file") => {
                let path_arg = self.val_to_string(&args[0]);
                let root = match &data {
                    CapabilityData::Fs { root } => root.clone(),
                    _ => unreachable!(),
                };
                match resolve_sandboxed_path(&root, &path_arg) {
                    Err(e) => Ok(Value::error(Value::string(e))),
                    Ok(full_path) => match std::fs::read_to_string(&full_path) {
                        Ok(content) => Ok(Value::string(content)),
                        Err(e) => Ok(Value::error(Value::string(format!("{}", e)))),
                    },
                }
            }
            ("Fs", "write_file") => {
                let path_arg = self.val_to_string(&args[0]);
                let content = self.val_to_string(&args[1]);
                let root = match &data {
                    CapabilityData::Fs { root } => root.clone(),
                    _ => unreachable!(),
                };
                let full_path = resolve_sandboxed_path(&root, &path_arg)?;
                if let Some(parent) = full_path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| format!("fs error: {}", e))?;
                    }
                }
                match std::fs::write(&full_path, content) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Ok(Value::error(Value::string(format!("{}", e)))),
                }
            }
            ("Fs", "read_dir") => {
                let path_arg = self.val_to_string(&args[0]);
                let root = match &data {
                    CapabilityData::Fs { root } => root.clone(),
                    _ => unreachable!(),
                };
                let full_path = resolve_sandboxed_path(&root, &path_arg)?;
                match std::fs::read_dir(&full_path) {
                    Ok(entries) => {
                        let mut names: Vec<Value> = Vec::new();
                        for e in entries.flatten() {
                            names.push(Value::string(e.file_name().to_string_lossy().to_string()));
                        }
                        names.sort_by(|a, b| {
                            let sa = if let Value::Heap(ha) = a {
                                if let HeapObject::String(s) = &*ha.borrow() {
                                    s.clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            let sb = if let Value::Heap(hb) = b {
                                if let HeapObject::String(s) = &*hb.borrow() {
                                    s.clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            sa.cmp(&sb)
                        });
                        Ok(Value::array(names))
                    }
                    Err(e) => Ok(Value::error(Value::string(format!("{}", e)))),
                }
            }
            ("Fs", "exists") => {
                let path_arg = self.val_to_string(&args[0]);
                let root = match &data {
                    CapabilityData::Fs { root } => root.clone(),
                    _ => unreachable!(),
                };
                match resolve_sandboxed_path(&root, &path_arg) {
                    Ok(full_path) => Ok(Value::bool(full_path.exists())),
                    Err(_) => Ok(Value::bool(false)),
                }
            }
            ("Fs", "mkdir") => {
                let path_arg = self.val_to_string(&args[0]);
                let root = match &data {
                    CapabilityData::Fs { root } => root.clone(),
                    _ => unreachable!(),
                };
                let full_path = resolve_sandboxed_path(&root, &path_arg)?;
                match std::fs::create_dir_all(&full_path) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Ok(Value::error(Value::string(format!("{}", e)))),
                }
            }
            ("Fs", "join") => {
                let mut result = PathBuf::new();
                for arg in args {
                    result.push(self.val_to_string(arg));
                }
                Ok(Value::string(result.to_string_lossy().to_string()))
            }
            ("Fs", "extension") => {
                let path_arg = self.val_to_string(&args[0]);
                let p = std::path::Path::new(&path_arg);
                Ok(match p.extension().and_then(|e| e.to_str()) {
                    Some(ext) => Value::string(ext.to_string()),
                    None => Value::Nil,
                })
            }
            ("Fs", "stem") => {
                let path_arg = self.val_to_string(&args[0]);
                let p = std::path::Path::new(&path_arg);
                Ok(match p.file_stem().and_then(|s| s.to_str()) {
                    Some(stem) => Value::string(stem.to_string()),
                    None => Value::Nil,
                })
            }
            ("Fs", "is_dir") => {
                let path_arg = self.val_to_string(&args[0]);
                let root = match &data {
                    CapabilityData::Fs { root } => root.clone(),
                    _ => unreachable!(),
                };
                match resolve_sandboxed_path(&root, &path_arg) {
                    Ok(full_path) => Ok(Value::bool(full_path.is_dir())),
                    Err(_) => Ok(Value::bool(false)),
                }
            }

            // === Env ===
            ("Env", "args") => {
                let env_args = match &data {
                    CapabilityData::Env { args: a } => a.clone(),
                    _ => unreachable!(),
                };
                Ok(Value::array(
                    env_args.into_iter().map(Value::string).collect(),
                ))
            }
            ("Env", "get") => {
                let key = self.val_to_string(&args[0]);
                match std::env::var(&key) {
                    Ok(val) => Ok(Value::string(val)),
                    Err(_) => Ok(Value::Nil),
                }
            }

            _ => Err(format!(
                "unknown method '{}' on capability '{}'",
                method, cap_name
            )),
        }
    }

    fn call_math(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "sqrt" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).sqrt(),
            )),
            "abs" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).abs(),
            )),
            "floor" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).floor(),
            )),
            "ceil" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).ceil(),
            )),
            "round" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).round(),
            )),
            "min" => {
                let (a, b) = (
                    args.first().map(|v| v.as_float()).unwrap_or(0.0),
                    args.get(1).map(|v| v.as_float()).unwrap_or(0.0),
                );
                Ok(Value::float(a.min(b)))
            }
            "max" => {
                let (a, b) = (
                    args.first().map(|v| v.as_float()).unwrap_or(0.0),
                    args.get(1).map(|v| v.as_float()).unwrap_or(0.0),
                );
                Ok(Value::float(a.max(b)))
            }
            "sin" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).sin(),
            )),
            "cos" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).cos(),
            )),
            "tan" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).tan(),
            )),
            "asin" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).asin(),
            )),
            "acos" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).acos(),
            )),
            "atan" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).atan(),
            )),
            "atan2" => {
                let (y, x) = (
                    args.first().map(|v| v.as_float()).unwrap_or(0.0),
                    args.get(1).map(|v| v.as_float()).unwrap_or(0.0),
                );
                Ok(Value::float(y.atan2(x)))
            }
            "log" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).ln(),
            )),
            "log2" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).log2(),
            )),
            "log10" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).log10(),
            )),
            "exp" => Ok(Value::float(
                args.first().map(|a| a.as_float()).unwrap_or(0.0).exp(),
            )),
            "pow" => {
                let (base, exp) = (
                    args.first().map(|v| v.as_float()).unwrap_or(0.0),
                    args.get(1).map(|v| v.as_float()).unwrap_or(0.0),
                );
                Ok(Value::float(base.powf(exp)))
            }
            "pi" => Ok(Value::float(std::f64::consts::PI)),
            "e" => Ok(Value::float(std::f64::consts::E)),
            "tau" => Ok(Value::float(std::f64::consts::TAU)),
            "inf" => Ok(Value::float(f64::INFINITY)),
            _ => Err(format!("unknown math function '{}'", name)),
        }
    }

    fn call_string_method(&self, s: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "split" => {
                let sep = self.val_to_string(args.first().unwrap_or(&Value::Nil));
                Ok(Value::array(
                    s.split(&sep)
                        .map(|p| Value::string(p.to_string()))
                        .collect(),
                ))
            }
            "trim" => Ok(Value::string(s.trim().to_string())),
            "trim_start" => Ok(Value::string(s.trim_start().to_string())),
            "trim_end" => Ok(Value::string(s.trim_end().to_string())),
            "replace" => {
                let f = self.val_to_string(&args[0]);
                let t = self.val_to_string(&args[1]);
                Ok(Value::string(s.replace(&f, &t)))
            }
            "find" => {
                let needle = self.val_to_string(&args[0]);
                match s.find(&needle) {
                    Some(bi) => Ok(Value::int(s[..bi].chars().count() as i64)),
                    None => Ok(Value::Nil),
                }
            }
            "substring" => {
                let start = args[0].as_int();
                let len = args[1].as_int();
                if start < 0 || len < 0 {
                    return Err(format!(
                        "substring: start ({}) and length ({}) must be non-negative",
                        start, len
                    ));
                }
                Ok(Value::string(
                    s.chars().skip(start as usize).take(len as usize).collect(),
                ))
            }
            "starts_with" => Ok(Value::bool(s.starts_with(&self.val_to_string(&args[0])))),
            "ends_with" => Ok(Value::bool(s.ends_with(&self.val_to_string(&args[0])))),
            "contains" => Ok(Value::bool(s.contains(&self.val_to_string(&args[0])))),
            "upper" => Ok(Value::string(s.to_uppercase())),
            "lower" => Ok(Value::string(s.to_lowercase())),
            "repeat" => {
                let n = args[0].as_int();
                if n < 0 {
                    return Err("negative repeat count".to_string());
                }
                if s.len() * n as usize > MAX_STRING_REPEAT {
                    return Err("string repeat too large".to_string());
                }
                Ok(Value::string(s.repeat(n as usize)))
            }
            "char_at" => {
                let mut i = args[0].as_int();
                let len = s.chars().count() as i64;
                if i < 0 {
                    i += len;
                }
                if i < 0 || i >= len {
                    return Err("char_at index out of bounds".to_string());
                }
                Ok(Value::string(
                    s.chars().nth(i as usize).unwrap().to_string(),
                ))
            }
            "index_of" => {
                if args.len() != 1 {
                    return Err("index_of takes 1 argument".into());
                }
                let needle = self.val_to_string(args.first().unwrap_or(&Value::Nil));
                Ok(Value::int(s.find(&needle).map(|i| i as i64).unwrap_or(-1)))
            }
            "last_index_of" => {
                if args.len() != 1 {
                    return Err("last_index_of takes 1 argument".into());
                }
                let needle = self.val_to_string(args.first().unwrap_or(&Value::Nil));
                Ok(Value::int(s.rfind(&needle).map(|i| i as i64).unwrap_or(-1)))
            }
            "slice" => {
                if args.len() != 2 {
                    return Err("slice takes 2 arguments".into());
                }
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len() as i64;
                let mut start = args[0].as_int();
                let mut end = args[1].as_int();
                if start < 0 {
                    start += len;
                }
                if end < 0 {
                    end += len;
                }
                let start = (start.max(0) as usize).min(chars.len());
                let end = (end.max(0) as usize).min(chars.len());
                if start >= end {
                    Ok(Value::string(String::new()))
                } else {
                    Ok(Value::string(chars[start..end].iter().collect()))
                }
            }
            "chars" => {
                if !args.is_empty() {
                    return Err("chars takes 0 arguments".into());
                }
                let char_strings: Vec<Value> =
                    s.chars().map(|c| Value::string(c.to_string())).collect();
                Ok(Value::array(char_strings))
            }
            "bytes" => {
                if !args.is_empty() {
                    return Err("bytes takes 0 arguments".into());
                }
                let byte_values: Vec<Value> = s.bytes().map(|b| Value::int(b as i64)).collect();
                Ok(Value::array(byte_values))
            }
            _ => Err(format!("unknown string method '{}'", method)),
        }
    }

    fn call_array_method(
        &mut self,
        href: &HeapRef,
        method: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match method {
            "push" => {
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    v.push(args[0].clone());
                }
                Ok(Value::Nil)
            }
            "pop" => {
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    v.pop().ok_or("pop from empty array".to_string())
                } else {
                    Err("pop on non-array".to_string())
                }
            }
            "insert" => {
                let i = args[0].as_int();
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    if i < 0 || i as usize > v.len() {
                        return Err(format!(
                            "insert index {} out of bounds (length {})",
                            i,
                            v.len()
                        ));
                    }
                    v.insert(i as usize, args[1].clone());
                }
                Ok(Value::Nil)
            }
            "remove" => {
                let i = args[0].as_int();
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    if i < 0 || i as usize >= v.len() {
                        return Err(format!(
                            "remove index {} out of bounds (length {})",
                            i,
                            v.len()
                        ));
                    }
                    v.remove(i as usize);
                }
                Ok(Value::Nil)
            }
            "sort" => {
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    // Determine sort type from first element
                    let is_string = v.first().is_some_and(|e| {
                        matches!(e, Value::Heap(h) if matches!(&*h.borrow(), HeapObject::String(_)))
                    });
                    if is_string {
                        v.sort_by(|a, b| {
                            let sa = a.display_as_string();
                            let sb = b.display_as_string();
                            sa.cmp(&sb)
                        });
                    } else {
                        v.sort_by_key(|a| a.as_int());
                    }
                }
                Ok(Value::Nil)
            }
            "reverse" => {
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    v.reverse();
                }
                Ok(Value::Nil)
            }
            "join" => {
                let sep = self.val_to_string(&args[0]);
                let o = href.borrow();
                if let HeapObject::Array(v) = &*o {
                    Ok(Value::string(
                        v.iter()
                            .map(|x| self.val_to_string(x))
                            .collect::<Vec<_>>()
                            .join(&sep),
                    ))
                } else {
                    Err("join on non-array".to_string())
                }
            }
            "contains" => {
                let o = href.borrow();
                if let HeapObject::Array(v) = &*o {
                    Ok(Value::bool(v.iter().any(|x| x.deep_eq(&args[0]))))
                } else {
                    Err("contains on non-array".to_string())
                }
            }
            "clone" => {
                let o = href.borrow();
                if let HeapObject::Array(v) = &*o {
                    Ok(Value::array(v.clone()))
                } else {
                    Err("clone on non-array".to_string())
                }
            }
            "map" => {
                let elements = {
                    let o = href.borrow();
                    if let HeapObject::Array(v) = &*o {
                        v.clone()
                    } else {
                        return Err("map on non-array".to_string());
                    }
                };
                let (func_idx, captures) = extract_closure(&args[0])?;
                let mut result = Vec::with_capacity(elements.len());
                for elem in elements {
                    let mut call_args = captures.clone();
                    call_args.push(elem);
                    result.push(self.call_function(func_idx, &call_args)?);
                }
                Ok(Value::array(result))
            }
            "filter" => {
                let elements = {
                    let o = href.borrow();
                    if let HeapObject::Array(v) = &*o {
                        v.clone()
                    } else {
                        return Err("filter on non-array".to_string());
                    }
                };
                let (func_idx, captures) = extract_closure(&args[0])?;
                let mut result = Vec::new();
                for elem in elements {
                    let mut call_args = captures.clone();
                    call_args.push(elem.clone());
                    let keep = self.call_function(func_idx, &call_args)?;
                    if keep.as_bool() {
                        result.push(elem);
                    }
                }
                Ok(Value::array(result))
            }
            "find" => {
                let elements = {
                    let o = href.borrow();
                    if let HeapObject::Array(v) = &*o {
                        v.clone()
                    } else {
                        return Err("find on non-array".to_string());
                    }
                };
                let (func_idx, captures) = extract_closure(&args[0])?;
                for elem in elements {
                    let mut call_args = captures.clone();
                    call_args.push(elem.clone());
                    let matched = self.call_function(func_idx, &call_args)?;
                    if matched.as_bool() {
                        return Ok(elem);
                    }
                }
                Ok(Value::Nil)
            }
            "any" => {
                let elements = {
                    let o = href.borrow();
                    if let HeapObject::Array(v) = &*o {
                        v.clone()
                    } else {
                        return Err("any on non-array".to_string());
                    }
                };
                let (func_idx, captures) = extract_closure(&args[0])?;
                for elem in elements {
                    let mut call_args = captures.clone();
                    call_args.push(elem);
                    if self.call_function(func_idx, &call_args)?.as_bool() {
                        return Ok(Value::bool(true));
                    }
                }
                Ok(Value::bool(false))
            }
            "all" => {
                let elements = {
                    let o = href.borrow();
                    if let HeapObject::Array(v) = &*o {
                        v.clone()
                    } else {
                        return Err("all on non-array".to_string());
                    }
                };
                let (func_idx, captures) = extract_closure(&args[0])?;
                for elem in elements {
                    let mut call_args = captures.clone();
                    call_args.push(elem);
                    if !self.call_function(func_idx, &call_args)?.as_bool() {
                        return Ok(Value::bool(false));
                    }
                }
                Ok(Value::bool(true))
            }
            _ => Err(format!("unknown array method '{}'", method)),
        }
    }

    fn call_map_method(
        &mut self,
        href: &HeapRef,
        method: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        match method {
            "get" => {
                let k = val_to_map_key(&args[0])?;
                let o = href.borrow();
                if let HeapObject::Map(m) = &*o {
                    Ok(m.get(&k).cloned().unwrap_or(Value::Nil))
                } else {
                    Err("get on non-map".to_string())
                }
            }
            "contains_key" => {
                let k = val_to_map_key(&args[0])?;
                let o = href.borrow();
                if let HeapObject::Map(m) = &*o {
                    Ok(Value::bool(m.contains_key(&k)))
                } else {
                    Err("contains_key on non-map".to_string())
                }
            }
            "keys" => {
                let o = href.borrow();
                if let HeapObject::Map(m) = &*o {
                    Ok(Value::array(m.keys().map(map_key_to_value).collect()))
                } else {
                    Err("keys on non-map".to_string())
                }
            }
            "values" => {
                let o = href.borrow();
                if let HeapObject::Map(m) = &*o {
                    Ok(Value::array(m.values().cloned().collect()))
                } else {
                    Err("values on non-map".to_string())
                }
            }
            "remove" => {
                let k = val_to_map_key(&args[0])?;
                let mut o = href.borrow_mut();
                if let HeapObject::Map(m) = &mut *o {
                    m.shift_remove(&k);
                }
                Ok(Value::Nil)
            }
            "clone" => {
                let o = href.borrow();
                if let HeapObject::Map(m) = &*o {
                    Ok(Value::map(m.clone()))
                } else {
                    Err("clone on non-map".to_string())
                }
            }
            _ => Err(format!("unknown map method '{}'", method)),
        }
    }
}

fn val_to_map_key(val: &Value) -> Result<MapKey, String> {
    match val {
        Value::Int(n) => Ok(MapKey::Int(*n)),
        Value::Bool(b) => Ok(MapKey::Bool(*b)),
        Value::Heap(href) => {
            let obj = href.borrow();
            if let HeapObject::String(s) = &*obj {
                Ok(MapKey::String(s.clone()))
            } else {
                Err("non-hashable map key".to_string())
            }
        }
        _ => Err("non-hashable map key".to_string()),
    }
}

fn checked_pow_i64(base: i64, exp: u64) -> Option<i64> {
    if exp == 0 {
        return Some(1);
    }
    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result.checked_mul(b)?;
        }
        e >>= 1;
        if e > 0 {
            b = b.checked_mul(b)?;
        }
    }
    Some(result)
}

fn narrow_int(val: i64, target: IntType) -> Result<i64, String> {
    match target {
        IntType::I8 => {
            if val < i8::MIN as i64 || val > i8::MAX as i64 {
                Err(format!("{} out of range for i8", val))
            } else {
                Ok(val)
            }
        }
        IntType::I16 => {
            if val < i16::MIN as i64 || val > i16::MAX as i64 {
                Err(format!("{} out of range for i16", val))
            } else {
                Ok(val)
            }
        }
        IntType::I32 => {
            if val < i32::MIN as i64 || val > i32::MAX as i64 {
                Err(format!("{} out of range for i32", val))
            } else {
                Ok(val)
            }
        }
        IntType::I64 => Ok(val),
        IntType::U8 => {
            if val < 0 || val > u8::MAX as i64 {
                Err(format!("{} out of range for u8", val))
            } else {
                Ok(val)
            }
        }
        IntType::U16 => {
            if val < 0 || val > u16::MAX as i64 {
                Err(format!("{} out of range for u16", val))
            } else {
                Ok(val)
            }
        }
        IntType::U32 => {
            if val < 0 || val > u32::MAX as i64 {
                Err(format!("{} out of range for u32", val))
            } else {
                Ok(val)
            }
        }
        IntType::U64 => {
            if val < 0 {
                Err(format!("{} out of range for u64", val))
            } else {
                Ok(val)
            }
        }
    }
}

/// Reference identity test. For value types (Int, Float, Bool, Nil), this
/// is equivalent to ==. For heap types, this checks pointer identity.
fn values_identical(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Nil, Value::Nil) => true,
        (Value::Heap(ha), Value::Heap(hb)) => Arc::ptr_eq(ha, hb),
        _ => false,
    }
}

fn value_is_type(val: &Value, type_name: &str) -> bool {
    match type_name {
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => {
            matches!(val, Value::Int(_))
        }
        "f64" => matches!(val, Value::Float(_)),
        "bool" => matches!(val, Value::Bool(_)),
        "nil" => val.is_nil(),
        "string" => {
            if let Value::Heap(href) = val {
                matches!(&*href.borrow(), HeapObject::String(_))
            } else {
                false
            }
        }
        name => {
            // Check for optional type: T?
            if let Some(inner) = name.strip_suffix('?') {
                return val.is_nil() || value_is_type(val, inner);
            }
            // Struct or enum name
            if let Value::Heap(href) = val {
                match &*href.borrow() {
                    HeapObject::Struct(s) => s.type_name == name,
                    HeapObject::Enum(e) => e.type_name == name,
                    HeapObject::Array(_) => name.starts_with('[') && name.ends_with(']'),
                    HeapObject::Map(..) => name.starts_with('[') && name.contains(':'),
                    HeapObject::Error(_) => false,
                    HeapObject::Range(_, _) => name == "Range",
                    _ => false,
                }
            } else {
                false
            }
        }
    }
}

/// Extract closure func_idx and captures from a Value.
/// Handles both heap-allocated closures (with captures) and plain function indices.
fn extract_closure(val: &Value) -> Result<(usize, Vec<Value>), String> {
    if let Some(href) = val.as_heap_ref() {
        let obj = href.borrow();
        if let HeapObject::Closure(closure) = &*obj {
            return Ok((closure.func_idx, closure.captures.clone()));
        }
    }
    // No-capture lambda: compiled as a plain integer function index
    if let Value::Int(idx) = val {
        return Ok((*idx as usize, Vec::new()));
    }
    Err("expected closure argument".to_string())
}

/// Resolve a path relative to a root directory, preventing escape via ..
fn resolve_sandboxed_path(root: &Path, user_path: &str) -> Result<PathBuf, String> {
    // Reject obvious escape attempts before any I/O
    let normalized = normalize_path_components(user_path);
    if normalized.starts_with("..") || normalized.contains("/../") || normalized.ends_with("/..") {
        return Err(format!("path '{}' escapes sandbox root", user_path));
    }

    let joined = root.join(user_path);
    // Canonicalize to resolve symlinks and verify containment
    let resolved = if joined.exists() {
        joined
            .canonicalize()
            .map_err(|e| format!("path error: {}", e))?
    } else {
        // For non-existent paths, canonicalize parent + filename
        let parent = joined.parent().unwrap_or(root);
        let parent_canonical = if parent.exists() {
            parent
                .canonicalize()
                .map_err(|e| format!("path error: {}", e))?
        } else {
            parent.to_path_buf()
        };
        parent_canonical.join(joined.file_name().unwrap_or_default())
    };
    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if !resolved.starts_with(&root_canonical) {
        return Err(format!("path '{}' escapes sandbox root", user_path));
    }
    Ok(resolved)
}

/// Normalize path components, collapsing foo/.. sequences.
fn normalize_path_components(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if parts.last().is_none_or(|p| *p == "..") {
                    parts.push("..");
                } else {
                    parts.pop();
                }
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
