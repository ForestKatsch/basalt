/// Basalt VM - Bytecode execution engine.
use crate::value::*;
use basalt_core::compiler::{CompiledFunction, IntType, Op, Program};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::sync::Arc;

const MAX_CALL_DEPTH: usize = 512;
const MAX_INSTRUCTIONS: u64 = 100_000_000;
const MAX_STRING_REPEAT: usize = 16 * 1024 * 1024;

pub struct VM {
    program: Program,
    call_depth: usize,
    instruction_count: u64,
    pub captured_output: Vec<String>,
    stdin_lines: Vec<String>,
    stdin_pos: usize,
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
        }
    }

    pub fn set_stdin(&mut self, lines: Vec<String>) {
        self.stdin_lines = lines;
    }

    pub fn run(&mut self) -> Result<Value, String> {
        let entry = self.program.entry_point;
        let main_func = &self.program.functions[entry];

        // Construct capability args for main
        let mut args = Vec::new();
        for pt in &main_func.param_types {
            match pt {
                basalt_core::types::Type::Capability(name) => {
                    // Capabilities are identified by a marker value
                    args.push(Value::int(match name.as_str() {
                        "Stdout" => 1,
                        "Stdin" => 2,
                        _ => 0,
                    }));
                }
                _ => args.push(Value::Nil),
            }
        }

        self.call_function(entry, &args)
    }

    fn call_function(&mut self, func_idx: usize, args: &[Value]) -> Result<Value, String> {
        self.call_depth += 1;
        if self.call_depth > MAX_CALL_DEPTH {
            return Err("stack overflow: maximum call depth exceeded".to_string());
        }

        let func = self.program.functions[func_idx].clone();
        let reg_count = (func.register_count as usize).max(256);
        let mut registers = vec![Value::Nil; reg_count];

        for (i, arg) in args.iter().enumerate() {
            if i < registers.len() {
                registers[i] = arg.clone();
            }
        }

        let result = self.execute(&func, &mut registers)?;
        self.call_depth -= 1;
        Ok(result)
    }

    fn execute(&mut self, func: &CompiledFunction, reg: &mut Vec<Value>) -> Result<Value, String> {
        let mut pc = 0;

        while pc < func.code.len() {
            self.instruction_count += 1;
            if self.instruction_count > MAX_INSTRUCTIONS {
                return Err("execution limit exceeded".to_string());
            }

            let op = &func.code[pc];
            pc += 1;

            match op {
                // === Constants ===
                Op::LoadInt(d, n) => {
                    self.ensure(reg, *d);
                    reg[*d as usize] = Value::int(*n);
                }
                Op::LoadFloat(d, f) => {
                    self.ensure(reg, *d);
                    reg[*d as usize] = Value::float(*f);
                }
                Op::LoadBool(d, b) => {
                    self.ensure(reg, *d);
                    reg[*d as usize] = Value::bool(*b);
                }
                Op::LoadString(d, idx) => {
                    self.ensure(reg, *d);
                    let s = self.program.strings[*idx as usize].clone();
                    reg[*d as usize] = Value::string(s);
                }
                Op::LoadNil(d) => {
                    self.ensure(reg, *d);
                    reg[*d as usize] = Value::Nil;
                }
                Op::Move(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let v = reg[*s as usize].clone();
                    reg[*d as usize] = v;
                }

                // === Integer Arithmetic ===
                Op::AddInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let va = reg[*a as usize].as_int();
                    let vb = reg[*b as usize].as_int();
                    reg[*d as usize] =
                        Value::int(va.checked_add(vb).ok_or("integer overflow in addition")?);
                }
                Op::SubInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let va = reg[*a as usize].as_int();
                    let vb = reg[*b as usize].as_int();
                    reg[*d as usize] = Value::int(
                        va.checked_sub(vb)
                            .ok_or("integer overflow in subtraction")?,
                    );
                }
                Op::MulInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let va = reg[*a as usize].as_int();
                    let vb = reg[*b as usize].as_int();
                    reg[*d as usize] = Value::int(
                        va.checked_mul(vb)
                            .ok_or("integer overflow in multiplication")?,
                    );
                }
                Op::DivInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let vb = reg[*b as usize].as_int();
                    if vb == 0 {
                        return Err("division by zero".to_string());
                    }
                    reg[*d as usize] = Value::int(reg[*a as usize].as_int() / vb);
                }
                Op::ModInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let vb = reg[*b as usize].as_int();
                    if vb == 0 {
                        return Err("modulo by zero".to_string());
                    }
                    reg[*d as usize] = Value::int(reg[*a as usize].as_int() % vb);
                }
                Op::PowInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let base = reg[*a as usize].as_int();
                    let exp = reg[*b as usize].as_int();
                    if exp < 0 {
                        return Err("negative exponent for integer power".to_string());
                    }
                    reg[*d as usize] = Value::int(
                        checked_pow_i64(base, exp as u64)
                            .ok_or("integer overflow in exponentiation")?,
                    );
                }
                Op::NegInt(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::int(
                        reg[*s as usize]
                            .as_int()
                            .checked_neg()
                            .ok_or("integer overflow in negation")?,
                    );
                }

                // === Float Arithmetic ===
                Op::AddFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::float(reg[*a as usize].as_float() + reg[*b as usize].as_float());
                }
                Op::SubFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::float(reg[*a as usize].as_float() - reg[*b as usize].as_float());
                }
                Op::MulFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::float(reg[*a as usize].as_float() * reg[*b as usize].as_float());
                }
                Op::DivFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::float(reg[*a as usize].as_float() / reg[*b as usize].as_float());
                }
                Op::ModFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::float(reg[*a as usize].as_float() % reg[*b as usize].as_float());
                }
                Op::PowFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::float(
                        reg[*a as usize]
                            .as_float()
                            .powf(reg[*b as usize].as_float()),
                    );
                }
                Op::NegFloat(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::float(-reg[*s as usize].as_float());
                }

                // === String ===
                Op::ConcatString(d, a, b) | Op::StringConcat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let sa = self.val_to_string(&reg[*a as usize]);
                    let sb = self.val_to_string(&reg[*b as usize]);
                    reg[*d as usize] = Value::string(sa + &sb);
                }

                // === Integer Comparisons ===
                Op::EqInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_int() == reg[*b as usize].as_int());
                }
                Op::NeqInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_int() != reg[*b as usize].as_int());
                }
                Op::LtInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_int() < reg[*b as usize].as_int());
                }
                Op::LteInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_int() <= reg[*b as usize].as_int());
                }
                Op::GtInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_int() > reg[*b as usize].as_int());
                }
                Op::GteInt(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_int() >= reg[*b as usize].as_int());
                }

                // === Float Comparisons ===
                Op::EqFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_float() == reg[*b as usize].as_float());
                }
                Op::NeqFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_float() != reg[*b as usize].as_float());
                }
                Op::LtFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_float() < reg[*b as usize].as_float());
                }
                Op::LteFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_float() <= reg[*b as usize].as_float());
                }
                Op::GtFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_float() > reg[*b as usize].as_float());
                }
                Op::GteFloat(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_float() >= reg[*b as usize].as_float());
                }

                // === String Comparisons ===
                Op::EqString(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(
                        self.val_to_string(&reg[*a as usize])
                            == self.val_to_string(&reg[*b as usize]),
                    );
                }
                Op::NeqString(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(
                        self.val_to_string(&reg[*a as usize])
                            != self.val_to_string(&reg[*b as usize]),
                    );
                }
                Op::LtString(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(
                        self.val_to_string(&reg[*a as usize])
                            < self.val_to_string(&reg[*b as usize]),
                    );
                }
                Op::LteString(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(
                        self.val_to_string(&reg[*a as usize])
                            <= self.val_to_string(&reg[*b as usize]),
                    );
                }
                Op::GtString(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(
                        self.val_to_string(&reg[*a as usize])
                            > self.val_to_string(&reg[*b as usize]),
                    );
                }
                Op::GteString(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(
                        self.val_to_string(&reg[*a as usize])
                            >= self.val_to_string(&reg[*b as usize]),
                    );
                }

                // === Bool Comparisons ===
                Op::EqBool(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_bool() == reg[*b as usize].as_bool());
                }
                Op::NeqBool(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_bool() != reg[*b as usize].as_bool());
                }

                // === Generic Equality ===
                Op::EqGeneric(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(reg[*a as usize].deep_eq(&reg[*b as usize]));
                }
                Op::NeqGeneric(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] = Value::bool(!reg[*a as usize].deep_eq(&reg[*b as usize]));
                }

                // === Logical ===
                Op::Not(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::bool(!reg[*s as usize].as_bool());
                }
                Op::And(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_bool() && reg[*b as usize].as_bool());
                }
                Op::Or(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::bool(reg[*a as usize].as_bool() || reg[*b as usize].as_bool());
                }

                // === Bitwise ===
                Op::BitAnd(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::int(reg[*a as usize].as_int() & reg[*b as usize].as_int());
                }
                Op::BitOr(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::int(reg[*a as usize].as_int() | reg[*b as usize].as_int());
                }
                Op::BitXor(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    reg[*d as usize] =
                        Value::int(reg[*a as usize].as_int() ^ reg[*b as usize].as_int());
                }
                Op::ShiftLeft(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let shift = reg[*b as usize].as_int();
                    if shift < 0 || shift > 63 {
                        return Err(format!("shift amount {} out of range", shift));
                    }
                    reg[*d as usize] = Value::int(reg[*a as usize].as_int() << shift);
                }
                Op::ShiftRight(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let shift = reg[*b as usize].as_int();
                    if shift < 0 || shift > 63 {
                        return Err(format!("shift amount {} out of range", shift));
                    }
                    reg[*d as usize] = Value::int(reg[*a as usize].as_int() >> shift);
                }

                // === Type Conversions ===
                Op::IntToFloat(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::float(reg[*s as usize].as_int() as f64);
                }
                Op::FloatToInt(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let f = reg[*s as usize].as_float();
                    if f.is_nan() || f.is_infinite() {
                        return Err("cannot convert NaN/Infinity to integer".to_string());
                    }
                    reg[*d as usize] = Value::int(f as i64);
                }
                Op::IntToString(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::string(reg[*s as usize].as_int().to_string());
                }
                Op::FloatToString(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::string(format_float_val(reg[*s as usize].as_float()));
                }
                Op::BoolToString(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::string(
                        if reg[*s as usize].as_bool() {
                            "true"
                        } else {
                            "false"
                        }
                        .to_string(),
                    );
                }
                Op::StringToInt(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let sv = self.val_to_string(&reg[*s as usize]);
                    reg[*d as usize] = Value::int(
                        sv.trim()
                            .parse::<i64>()
                            .map_err(|_| format!("cannot convert '{}' to i64", sv))?,
                    );
                }
                Op::StringToFloat(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let sv = self.val_to_string(&reg[*s as usize]);
                    reg[*d as usize] = Value::float(
                        sv.trim()
                            .parse::<f64>()
                            .map_err(|_| format!("cannot convert '{}' to f64", sv))?,
                    );
                }
                Op::StringToIntSafe(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let sv = self.val_to_string(&reg[*s as usize]);
                    reg[*d as usize] = match sv.trim().parse::<i64>() {
                        Ok(n) => Value::int(n),
                        Err(_) => Value::Nil,
                    };
                }
                Op::StringToFloatSafe(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let sv = self.val_to_string(&reg[*s as usize]);
                    reg[*d as usize] = match sv.trim().parse::<f64>() {
                        Ok(f) => Value::float(f),
                        Err(_) => Value::Nil,
                    };
                }
                Op::IntNarrow(d, s, it) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::int(narrow_int(reg[*s as usize].as_int(), *it)?);
                }
                Op::IntNarrowSafe(d, s, it) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = match narrow_int(reg[*s as usize].as_int(), *it) {
                        Ok(v) => Value::int(v),
                        Err(_) => Value::Nil,
                    };
                }
                Op::IntWiden(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = reg[*s as usize].clone();
                }

                // === Control Flow ===
                Op::Jump(off) => {
                    pc = (pc as i32 + off - 1) as usize;
                }
                Op::JumpIfTrue(r, off) => {
                    self.ensure(reg, *r);
                    if reg[*r as usize].as_bool() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfFalse(r, off) => {
                    self.ensure(reg, *r);
                    if !reg[*r as usize].as_bool() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfNil(r, off) => {
                    self.ensure(reg, *r);
                    if reg[*r as usize].is_nil() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfNotNil(r, off) => {
                    self.ensure(reg, *r);
                    if !reg[*r as usize].is_nil() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }
                Op::JumpIfError(r, off) => {
                    self.ensure(reg, *r);
                    if reg[*r as usize].is_error() {
                        pc = (pc as i32 + off - 1) as usize;
                    }
                }

                // === Function Calls ===
                Op::Call(d, fr, ac) => {
                    self.ensure(reg, *d);
                    self.ensure(reg, *fr);
                    let func_idx = reg[*fr as usize].as_int() as usize;
                    let mut args = Vec::new();
                    let start = *fr as usize + 1;
                    for i in 0..*ac as usize {
                        if start + i < reg.len() {
                            args.push(reg[start + i].clone());
                        }
                    }
                    if func_idx < self.program.functions.len() {
                        reg[*d as usize] = self.call_function(func_idx, &args)?;
                    } else {
                        reg[*d as usize] = Value::Nil;
                    }
                }
                Op::Return(r) => {
                    self.ensure(reg, *r);
                    return Ok(reg[*r as usize].clone());
                }
                Op::ReturnNil => {
                    return Ok(Value::Nil);
                }
                Op::ReturnError(r) => {
                    self.ensure(reg, *r);
                    return Ok(reg[*r as usize].clone());
                }

                // === Collections ===
                Op::MakeArray(d, start, count) => {
                    self.ensure(reg, *d);
                    let mut vals = Vec::new();
                    for i in 0..*count {
                        let idx = *start as usize + i as usize;
                        if idx < reg.len() {
                            vals.push(reg[idx].clone());
                        }
                    }
                    reg[*d as usize] = Value::array(vals);
                }
                Op::MakeMap(d, start, count) => {
                    self.ensure(reg, *d);
                    let mut map = IndexMap::new();
                    for i in 0..*count {
                        let ki = *start as usize + (i as usize * 2);
                        let vi = ki + 1;
                        if ki < reg.len() && vi < reg.len() {
                            let key = val_to_map_key(&reg[ki])?;
                            map.insert(key, reg[vi].clone());
                        }
                    }
                    reg[*d as usize] = Value::map(map);
                }
                Op::MakeTuple(d, start, count) => {
                    self.ensure(reg, *d);
                    let mut vals = Vec::new();
                    for i in 0..*count {
                        let idx = *start as usize + i as usize;
                        if idx < reg.len() {
                            vals.push(reg[idx].clone());
                        }
                    }
                    reg[*d as usize] = Value::tuple(vals);
                }

                // === Struct ===
                Op::MakeStruct(d, tid, fc) => {
                    self.ensure(reg, *d);
                    let tn = self.program.type_ids[*tid as usize].clone();
                    let start = (*d as usize).saturating_sub(*fc as usize);
                    let mut fields = Vec::new();
                    for i in 0..*fc as usize {
                        if start + i < reg.len() {
                            fields.push(reg[start + i].clone());
                        }
                    }
                    reg[*d as usize] = Value::new_struct(tn, fields);
                }
                Op::GetField(d, o, fi) => {
                    self.ensure(reg, (*d).max(*o));
                    let val = if let Some(href) = reg[*o as usize].as_heap_ref().cloned() {
                        let obj = href.borrow();
                        match &*obj {
                            HeapObject::Struct(s) => {
                                s.fields.get(*fi as usize).cloned().unwrap_or(Value::Nil)
                            }
                            HeapObject::Tuple(vals) => {
                                vals.get(*fi as usize).cloned().unwrap_or(Value::Nil)
                            }
                            _ => return Err("GetField on non-struct".to_string()),
                        }
                    } else {
                        return Err("GetField on non-heap value".to_string());
                    };
                    reg[*d as usize] = val;
                }
                Op::SetField(o, fi, v) => {
                    self.ensure(reg, (*o).max(*v));
                    let value = reg[*v as usize].clone();
                    if let Some(href) = reg[*o as usize].as_heap_ref() {
                        let mut obj = href.borrow_mut();
                        if let HeapObject::Struct(s) = &mut *obj {
                            if (*fi as usize) < s.fields.len() {
                                s.fields[*fi as usize] = value;
                            }
                        }
                    }
                }

                // === Enum ===
                Op::MakeEnum(d, tid, vi, fc) => {
                    self.ensure(reg, *d);
                    let tn = self.program.type_ids[*tid as usize].clone();
                    let start = (*d as usize).saturating_sub(*fc as usize);
                    let mut fields = Vec::new();
                    for i in 0..*fc as usize {
                        if start + i < reg.len() {
                            fields.push(reg[start + i].clone());
                        }
                    }
                    reg[*d as usize] = Value::new_enum(tn, *vi, fields);
                }
                Op::GetEnumTag(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let val = if let Some(href) = reg[*s as usize].as_heap_ref().cloned() {
                        let obj = href.borrow();
                        if let HeapObject::Enum(e) = &*obj {
                            Value::int(e.variant_index as i64)
                        } else {
                            Value::int(-1)
                        }
                    } else {
                        Value::int(-1)
                    };
                    reg[*d as usize] = val;
                }
                Op::GetEnumField(d, s, fi) => {
                    self.ensure(reg, (*d).max(*s));
                    let val = if let Some(href) = reg[*s as usize].as_heap_ref().cloned() {
                        let obj = href.borrow();
                        if let HeapObject::Enum(e) = &*obj {
                            e.fields.get(*fi as usize).cloned().unwrap_or(Value::Nil)
                        } else {
                            Value::Nil
                        }
                    } else {
                        Value::Nil
                    };
                    reg[*d as usize] = val;
                }

                // === Index ===
                Op::GetIndex(d, o, i) => {
                    self.ensure3(reg, *d, *o, *i);
                    reg[*d as usize] = self.get_index(&reg[*o as usize], &reg[*i as usize])?;
                }
                Op::SetIndex(o, i, v) => {
                    self.ensure3(reg, *o, *i, *v);
                    self.set_index(
                        &reg[*o as usize],
                        &reg[*i as usize],
                        reg[*v as usize].clone(),
                    )?;
                }
                Op::ArrayLen(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let val = if let Some(href) = reg[*s as usize].as_heap_ref().cloned() {
                        let obj = href.borrow();
                        if let HeapObject::Array(v) = &*obj {
                            Value::int(v.len() as i64)
                        } else {
                            Value::int(0)
                        }
                    } else {
                        Value::int(0)
                    };
                    reg[*d as usize] = val;
                }
                Op::StringLen(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let sv = self.val_to_string(&reg[*s as usize]);
                    reg[*d as usize] = Value::int(sv.chars().count() as i64);
                }
                Op::MapLen(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let val = if let Some(href) = reg[*s as usize].as_heap_ref().cloned() {
                        let obj = href.borrow();
                        if let HeapObject::Map(m) = &*obj {
                            Value::int(m.len() as i64)
                        } else {
                            Value::int(0)
                        }
                    } else {
                        Value::int(0)
                    };
                    reg[*d as usize] = val;
                }

                // === Method Calls ===
                Op::CallMethod(d, o, mid, ac) => {
                    self.ensure(reg, (*d).max(*o));
                    let method = self.program.method_names[*mid as usize].clone();
                    let mut args = Vec::new();
                    let start = *o as usize + 1;
                    for i in 0..*ac as usize {
                        if start + i < reg.len() {
                            args.push(reg[start + i].clone());
                        }
                    }
                    let obj = reg[*o as usize].clone();
                    reg[*d as usize] = self.call_method(&obj, &method, &args)?;
                }
                Op::CallCapability(d, c, mid, ac) => {
                    self.ensure(reg, (*d).max(*c));
                    let method = self.program.method_names[*mid as usize].clone();
                    let mut args = Vec::new();
                    let start = *c as usize + 1;
                    for i in 0..*ac as usize {
                        if start + i < reg.len() {
                            args.push(reg[start + i].clone());
                        }
                    }
                    reg[*d as usize] = self.call_capability(&method, &args)?;
                }

                // === Error Handling ===
                Op::MakeError(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let v = reg[*s as usize].clone();
                    reg[*d as usize] = Value::error(v);
                }
                Op::UnwrapError(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let val = if let Some(href) = reg[*s as usize].as_heap_ref().cloned() {
                        let obj = href.borrow();
                        if let HeapObject::Error(inner) = &*obj {
                            (**inner).clone()
                        } else {
                            reg[*s as usize].clone()
                        }
                    } else {
                        reg[*s as usize].clone()
                    };
                    reg[*d as usize] = val;
                }
                Op::IsError(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::bool(reg[*s as usize].is_error());
                }
                Op::IsNil(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::bool(reg[*s as usize].is_nil());
                }

                // === Type Testing ===
                Op::IsType(d, s, _tid) => {
                    self.ensure(reg, (*d).max(*s));
                    reg[*d as usize] = Value::bool(false); /* TODO */
                }
                Op::IsEnumVariant(d, s, vi) => {
                    self.ensure(reg, (*d).max(*s));
                    let is_match = if let Some(href) = reg[*s as usize].as_heap_ref() {
                        let obj = href.borrow();
                        if let HeapObject::Enum(e) = &*obj {
                            e.variant_index == *vi
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    reg[*d as usize] = Value::bool(is_match);
                }
                Op::IsIdentical(d, a, b) => {
                    self.ensure3(reg, *d, *a, *b);
                    let identical = match (&reg[*a as usize], &reg[*b as usize]) {
                        (Value::Heap(a), Value::Heap(b)) => Arc::ptr_eq(a, b),
                        (a, b) => a.deep_eq(b),
                    };
                    reg[*d as usize] = Value::bool(identical);
                }

                // === Range ===
                Op::MakeRange(d, s, e) => {
                    self.ensure3(reg, *d, *s, *e);
                    reg[*d as usize] =
                        Value::range(reg[*s as usize].as_int(), reg[*e as usize].as_int());
                }

                // === Iterators ===
                Op::IterInit(d, s) => {
                    self.ensure(reg, (*d).max(*s));
                    let iter = self.create_iterator(&reg[*s as usize])?;
                    reg[*d as usize] =
                        Value::Heap(Arc::new(RefCell::new(HeapObject::Iterator(iter))));
                }
                Op::IterNext(vd, dd, ir) => {
                    self.ensure(reg, (*vd).max(*dd).max(*ir));
                    if let Some(href) = reg[*ir as usize].as_heap_ref().cloned() {
                        let mut obj = href.borrow_mut();
                        if let HeapObject::Iterator(iter) = &mut *obj {
                            match iter {
                                IterState::Array { values, index } => {
                                    if *index < values.len() {
                                        reg[*vd as usize] = values[*index].clone();
                                        reg[*dd as usize] = Value::bool(false);
                                        *index += 1;
                                    } else {
                                        reg[*dd as usize] = Value::bool(true);
                                    }
                                }
                                IterState::String { chars, index } => {
                                    if *index < chars.len() {
                                        reg[*vd as usize] = Value::string(chars[*index].clone());
                                        reg[*dd as usize] = Value::bool(false);
                                        *index += 1;
                                    } else {
                                        reg[*dd as usize] = Value::bool(true);
                                    }
                                }
                                IterState::Range { current, end } => {
                                    if *current < *end {
                                        reg[*vd as usize] = Value::int(*current);
                                        reg[*dd as usize] = Value::bool(false);
                                        *current += 1;
                                    } else {
                                        reg[*dd as usize] = Value::bool(true);
                                    }
                                }
                                IterState::Map {
                                    keys,
                                    values: _,
                                    index,
                                } => {
                                    if *index < keys.len() {
                                        reg[*vd as usize] = map_key_to_value(&keys[*index]);
                                        reg[*dd as usize] = Value::bool(false);
                                        *index += 1;
                                    } else {
                                        reg[*dd as usize] = Value::bool(true);
                                    }
                                }
                            }
                        }
                    }
                }
                Op::IterNextKV(kd, vd, dd, ir) => {
                    self.ensure(reg, (*kd).max(*vd).max(*dd).max(*ir));
                    if let Some(href) = reg[*ir as usize].as_heap_ref().cloned() {
                        let mut obj = href.borrow_mut();
                        if let HeapObject::Iterator(iter) = &mut *obj {
                            match iter {
                                IterState::Array { values, index } => {
                                    if *index < values.len() {
                                        reg[*kd as usize] = values[*index].clone();
                                        reg[*vd as usize] = Value::int(*index as i64);
                                        reg[*dd as usize] = Value::bool(false);
                                        *index += 1;
                                    } else {
                                        reg[*dd as usize] = Value::bool(true);
                                    }
                                }
                                IterState::Map {
                                    keys,
                                    values,
                                    index,
                                } => {
                                    if *index < keys.len() {
                                        reg[*kd as usize] = map_key_to_value(&keys[*index]);
                                        reg[*vd as usize] = values[*index].clone();
                                        reg[*dd as usize] = Value::bool(false);
                                        *index += 1;
                                    } else {
                                        reg[*dd as usize] = Value::bool(true);
                                    }
                                }
                                _ => {
                                    reg[*dd as usize] = Value::bool(true);
                                }
                            }
                        }
                    }
                }

                // === Misc ===
                Op::Panic(r) => {
                    self.ensure(reg, *r);
                    return Err(format!("panic: {}", self.val_to_string(&reg[*r as usize])));
                }
                Op::Nop => {}
                Op::Halt => {
                    return Ok(Value::Nil);
                }

                // Remaining ops
                _ => {
                    return Err(format!("unimplemented op: {:?}", op));
                }
            }
        }

        Ok(Value::Nil)
    }

    fn ensure(&self, reg: &mut Vec<Value>, r: u16) {
        while reg.len() <= r as usize {
            reg.push(Value::Nil);
        }
    }

    fn ensure3(&self, reg: &mut Vec<Value>, a: u16, b: u16, c: u16) {
        self.ensure(reg, a.max(b).max(c));
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
        if let Some(href) = obj.as_heap_ref() {
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
                _ => Err("cannot index into this value".to_string()),
            }
        } else {
            Err("cannot index into non-heap value".to_string())
        }
    }

    fn set_index(&self, obj: &Value, idx: &Value, val: Value) -> Result<(), String> {
        if let Some(href) = obj.as_heap_ref() {
            let mut o = href.borrow_mut();
            match &mut *o {
                HeapObject::Array(vals) => {
                    let mut i = idx.as_int();
                    if i < 0 {
                        i += vals.len() as i64;
                    }
                    if i < 0 || i as usize >= vals.len() {
                        return Err(format!("array index out of bounds"));
                    }
                    vals[i as usize] = val;
                    Ok(())
                }
                HeapObject::Map(entries) => {
                    let key = val_to_map_key(idx)?;
                    entries.insert(key, val);
                    Ok(())
                }
                _ => Err("cannot set index on this value".to_string()),
            }
        } else {
            Err("cannot set index on non-heap value".to_string())
        }
    }

    fn create_iterator(&self, val: &Value) -> Result<IterState, String> {
        if let Some(href) = val.as_heap_ref() {
            let obj = href.borrow();
            match &*obj {
                HeapObject::Array(v) => Ok(IterState::Array {
                    values: v.clone(),
                    index: 0,
                }),
                HeapObject::Map(m) => Ok(IterState::Map {
                    keys: m.keys().cloned().collect(),
                    values: m.values().cloned().collect(),
                    index: 0,
                }),
                HeapObject::String(s) => Ok(IterState::String {
                    chars: s.chars().map(|c| c.to_string()).collect(),
                    index: 0,
                }),
                HeapObject::Range(s, e) => Ok(IterState::Range {
                    current: *s,
                    end: *e,
                }),
                _ => Err("cannot iterate over this value".to_string()),
            }
        } else {
            Err("cannot iterate over non-iterable value".to_string())
        }
    }

    fn call_method(&mut self, obj: &Value, method: &str, args: &[Value]) -> Result<Value, String> {
        // Module.function calls
        if method.contains('.') {
            let parts: Vec<&str> = method.splitn(2, '.').collect();
            if parts[0] == "math" {
                return self.call_math(parts[1], args);
            }
            // Look up compiled function by name
            for (i, f) in self.program.functions.iter().enumerate() {
                if f.name == parts[1] {
                    return self.call_function(i, args);
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
                    // Look for user methods
                    for (i, f) in self.program.functions.iter().enumerate() {
                        if f.name == method {
                            let mut call_args = vec![obj.clone()];
                            call_args.extend_from_slice(args);
                            return self.call_function(i, &call_args);
                        }
                    }
                    return Err(format!("unknown method '{}' on '{}'", method, type_name));
                }
                _ => {}
            }
        }
        Err(format!("cannot call method '{}' on this value", method))
    }

    fn call_capability(&mut self, method: &str, args: &[Value]) -> Result<Value, String> {
        match method {
            "println" => {
                let text = if !args.is_empty() {
                    self.val_to_string(&args[0])
                } else {
                    String::new()
                };
                self.captured_output.push(text.clone());
                println!("{}", text);
                Ok(Value::Nil)
            }
            "print" => {
                let text = if !args.is_empty() {
                    self.val_to_string(&args[0])
                } else {
                    String::new()
                };
                self.captured_output.push(text.clone());
                print!("{}", text);
                Ok(Value::Nil)
            }
            "flush" => {
                use std::io::Write;
                std::io::stdout().flush().ok();
                Ok(Value::Nil)
            }
            "read_line" => {
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
            _ => Err(format!("unknown capability method '{}'", method)),
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
                    args.get(0).map(|v| v.as_float()).unwrap_or(0.0),
                    args.get(1).map(|v| v.as_float()).unwrap_or(0.0),
                );
                Ok(Value::float(a.min(b)))
            }
            "max" => {
                let (a, b) = (
                    args.get(0).map(|v| v.as_float()).unwrap_or(0.0),
                    args.get(1).map(|v| v.as_float()).unwrap_or(0.0),
                );
                Ok(Value::float(a.max(b)))
            }
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
                let start = args[0].as_int() as usize;
                let len = args[1].as_int() as usize;
                Ok(Value::string(s.chars().skip(start).take(len).collect()))
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
                    return Err(format!("char_at index out of bounds"));
                }
                Ok(Value::string(
                    s.chars().nth(i as usize).unwrap().to_string(),
                ))
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
                let i = args[0].as_int() as usize;
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    v.insert(i, args[1].clone());
                }
                Ok(Value::Nil)
            }
            "remove" => {
                let i = args[0].as_int() as usize;
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    v.remove(i);
                }
                Ok(Value::Nil)
            }
            "sort" => {
                let mut o = href.borrow_mut();
                if let HeapObject::Array(v) = &mut *o {
                    v.sort_by(|a, b| a.as_int().cmp(&b.as_int()));
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

fn format_float_val(f: f64) -> String {
    if f == f.floor() && f.is_finite() && f.abs() < 1e15 {
        format!("{:.1}", f)
    } else {
        format!("{}", f)
    }
}
