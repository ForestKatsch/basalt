use crate::ast::{BinOp, Pattern, PatternKind, UnaryOp};
use super::*;
use super::capture::*;
use std::collections::HashMap;

pub fn compile_program(program: &TypedProgram) -> Result<Program, String> {
    let mut compiler = Compiler::new(program.type_info.clone());
    compiler.compile_program(program)
}

struct Compiler {
    functions: Vec<CompiledFunction>,
    strings: Vec<String>,
    method_names: Vec<String>,
    type_ids: Vec<String>,
    type_info: TypeInfo,
    globals: Vec<(String, Type)>,
}

struct FnCompiler {
    code: Vec<Op>,
    line_table: Vec<u32>,
    current_line: u32,
    registers: u16,
    locals: HashMap<String, u16>, // name -> register
    local_types: HashMap<String, Type>,
    #[allow(clippy::type_complexity)]
    scope_stack: Vec<Vec<(String, Option<u16>, Option<Type>)>>, // (name, prev_reg, prev_type)
    loop_breaks: Vec<Vec<usize>>,    // jump indices to patch on break
    loop_continues: Vec<Vec<usize>>, // jump indices to patch on continue
    loop_starts: Vec<usize>,
    /// Variables that are captured by closures — stored in heap cells.
    /// Reads/writes to these go through CellGet/CellSet.
    captured_vars: std::collections::HashSet<String>,
}

impl FnCompiler {
    fn new() -> Self {
        FnCompiler {
            code: Vec::new(),
            line_table: Vec::new(),
            current_line: 0,
            registers: 0,
            locals: HashMap::new(),
            local_types: HashMap::new(),
            scope_stack: vec![Vec::new()],
            loop_breaks: Vec::new(),
            loop_continues: Vec::new(),
            loop_starts: Vec::new(),
            captured_vars: std::collections::HashSet::new(),
        }
    }

    fn alloc_reg(&mut self) -> u16 {
        let r = self.registers;
        self.registers += 1;
        r
    }

    fn emit(&mut self, op: Op) -> usize {
        let idx = self.code.len();
        self.code.push(op);
        self.line_table.push(self.current_line);
        idx
    }

    fn current_offset(&self) -> usize {
        self.code.len()
    }

    fn patch_jump(&mut self, idx: usize, target: usize) {
        let offset = target as i32 - idx as i32;
        match &mut self.code[idx] {
            Op::Jump(ref mut o) => *o = offset,
            Op::JumpIfTrue(_, ref mut o) => *o = offset,
            Op::JumpIfFalse(_, ref mut o) => *o = offset,
            Op::JumpIfNil(_, ref mut o) => *o = offset,
            Op::JumpIfNotNil(_, ref mut o) => *o = offset,
            Op::JumpIfError(_, ref mut o) => *o = offset,
            _ => {}
        }
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        if let Some(entries) = self.scope_stack.pop() {
            for (name, prev_reg, prev_ty) in entries {
                if let Some(reg) = prev_reg {
                    // Restore the previous binding
                    self.locals.insert(name.clone(), reg);
                    if let Some(ty) = prev_ty {
                        self.local_types.insert(name, ty);
                    }
                } else {
                    // No previous binding — remove entirely
                    self.locals.remove(&name);
                    self.local_types.remove(&name);
                }
            }
        }
    }

    fn declare_local(&mut self, name: &str, ty: &Type) -> u16 {
        let reg = self.alloc_reg();
        // Save previous binding (if any) for restoration on scope pop
        let prev_reg = self.locals.insert(name.to_string(), reg);
        let prev_ty = self.local_types.insert(name.to_string(), ty.clone());
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.push((name.to_string(), prev_reg, prev_ty));
        }
        reg
    }
}

impl Compiler {
    fn new(type_info: TypeInfo) -> Self {
        Compiler {
            functions: Vec::new(),
            strings: Vec::new(),
            method_names: Vec::new(),
            type_ids: Vec::new(),
            type_info,
            globals: Vec::new(),
        }
    }

    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(idx) = self.strings.iter().position(|x| x == s) {
            return idx as u32;
        }
        let idx = self.strings.len() as u32;
        self.strings.push(s.to_string());
        idx
    }

    fn intern_method(&mut self, name: &str) -> u32 {
        if let Some(idx) = self.method_names.iter().position(|x| x == name) {
            return idx as u32;
        }
        let idx = self.method_names.len() as u32;
        self.method_names.push(name.to_string());
        idx
    }

    fn intern_type(&mut self, name: &str) -> u32 {
        if let Some(idx) = self.type_ids.iter().position(|x| x == name) {
            return idx as u32;
        }
        let idx = self.type_ids.len() as u32;
        self.type_ids.push(name.to_string());
        idx
    }

    fn compile_program(&mut self, program: &TypedProgram) -> Result<Program, String> {
        // First, compile all top-level let declarations as globals
        for item in &program.items {
            if let TypedItem::Let(decl) = item {
                self.globals.push((decl.name.clone(), decl.ty.clone()));
            }
        }

        // Pre-register all functions so forward references resolve
        let mut fn_defs: Vec<&TypedFnDef> = Vec::new();
        for item in &program.items {
            if let TypedItem::Function(fdef) = item {
                let idx = self.functions.len();
                self.functions.push(CompiledFunction {
                    name: fdef.name.clone(),
                    param_count: fdef.params.len() as u8,
                    register_count: 0,
                    code: Vec::new(),
                    param_types: fdef.params.iter().map(|(_, t)| t.clone()).collect(),
                    return_type: fdef.return_type.clone(),
                    line_table: Vec::new(),
                });
                fn_defs.push(fdef);
                if fdef.name == "main" {
                    // entry_point assigned below after compile
                }
                let _ = idx;
            }
        }

        // Now compile all function bodies (forward references work via self.functions)
        let mut entry_point = None;
        for fdef in &fn_defs {
            // SAFETY: fdef.name was pre-registered in the loop above
            let idx = self.functions.iter().position(|f| f.name == fdef.name).unwrap();
            self.compile_fn_body(idx, fdef)?;
            if fdef.name == "main" {
                entry_point = Some(idx);
            }
        }

        // Note: Struct/enum method bodies are type-checked and compiled
        // as part of the type definition processing in check_item.
        // Methods are registered as TypedFnDefs in the typed program items.

        let entry = entry_point.ok_or("entry module must define a `main` function")?;

        let functions = std::mem::take(&mut self.functions);
        let mut method_lookup: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, f) in functions.iter().enumerate() {
            method_lookup.entry(f.name.clone()).or_default().push(i);
        }

        Ok(Program {
            functions,
            strings: std::mem::take(&mut self.strings),
            entry_point: entry,
            type_info: std::mem::take(&mut self.type_info),
            method_names: std::mem::take(&mut self.method_names),
            type_ids: std::mem::take(&mut self.type_ids),
            globals: std::mem::take(&mut self.globals),
            method_lookup,
        })
    }

    fn compile_fn(
        &mut self,
        fdef: &TypedFnDef,
        cell_params: &[String],
    ) -> Result<usize, String> {
        // Register a placeholder then compile the body.
        let idx = self.functions.len();
        self.functions.push(CompiledFunction {
            name: fdef.name.clone(),
            param_count: fdef.params.len() as u8,
            register_count: 0,
            code: Vec::new(),
            param_types: fdef.params.iter().map(|(_, t)| t.clone()).collect(),
            return_type: fdef.return_type.clone(),
            line_table: Vec::new(),
        });
        self.compile_fn_body_with_cells(idx, fdef, cell_params)?;
        Ok(idx)
    }

    fn compile_fn_body(&mut self, idx: usize, fdef: &TypedFnDef) -> Result<(), String> {
        self.compile_fn_body_with_cells(idx, fdef, &[])
    }

    fn compile_fn_body_with_cells(
        &mut self,
        idx: usize,
        fdef: &TypedFnDef,
        cell_params: &[String],
    ) -> Result<(), String> {
        let mut fc = FnCompiler::new();

        // Pre-scan: identify variables captured by lambdas in this function
        let captured = find_captured_vars_in_block(&fdef.body, &fdef.params);
        fc.captured_vars = captured;

        // Also mark parameters that arrive as cells (from outer closure capture)
        for name in cell_params {
            fc.captured_vars.insert(name.clone());
        }

        // Allocate registers for parameters
        for (name, ty) in &fdef.params {
            fc.declare_local(name, ty);
            // If this param is captured by inner lambdas AND not already a cell,
            // wrap it in a cell immediately
            if fc.captured_vars.contains(name) && !cell_params.contains(name) {
                // SAFETY: name was just declared via declare_local above
                let reg = *fc.locals.get(name).unwrap();
                fc.emit(Op::MakeCell(reg, reg));
            }
        }

        // Compile body
        self.compile_block(&mut fc, &fdef.body)?;

        // Ensure function ends with a return
        if !matches!(
            fc.code.last(),
            Some(Op::Return(_)) | Some(Op::ReturnNil) | Some(Op::ReturnError(_))
        ) {
            fc.emit(Op::ReturnNil);
        }

        // Fill in the placeholder with the actual compiled function
        self.functions[idx] = CompiledFunction {
            name: fdef.name.clone(),
            param_count: fdef.params.len() as u8,
            register_count: fc.registers,
            code: fc.code,
            param_types: fdef.params.iter().map(|(_, t)| t.clone()).collect(),
            return_type: fdef.return_type.clone(),
            line_table: fc.line_table,
        };

        Ok(())
    }

    fn compile_block(
        &mut self,
        fc: &mut FnCompiler,
        block: &TypedBlock,
    ) -> Result<Option<u16>, String> {
        fc.push_scope();
        let mut last_reg = None;
        for stmt in &block.stmts {
            last_reg = self.compile_stmt(fc, stmt)?;
        }
        fc.pop_scope();
        Ok(last_reg)
    }

    fn compile_stmt(
        &mut self,
        fc: &mut FnCompiler,
        stmt: &TypedStmt,
    ) -> Result<Option<u16>, String> {
        match stmt {
            TypedStmt::Let(decl) => {
                let value_reg = self.compile_expr(fc, &decl.value)?;
                let local_reg = fc.declare_local(&decl.name, &decl.ty);
                if fc.captured_vars.contains(&decl.name) {
                    // Wrap in a capture cell for by-reference sharing
                    fc.emit(Op::MakeCell(local_reg, value_reg));
                } else {
                    fc.emit(Op::Move(local_reg, value_reg));
                }
                Ok(None)
            }
            TypedStmt::LetTuple(bindings, value) => {
                let tuple_reg = self.compile_expr(fc, value)?;
                for (i, (name, ty)) in bindings.iter().enumerate() {
                    let local_reg = fc.declare_local(name, ty);
                    fc.emit(Op::GetField(local_reg, tuple_reg, i as u16));
                }
                Ok(None)
            }
            TypedStmt::Assign(target, value) => {
                let value_reg = self.compile_expr(fc, value)?;
                match &**target {
                    TypedAssignTarget::Variable(name, _) => {
                        if let Some(&reg) = fc.locals.get(name) {
                            if fc.captured_vars.contains(name) {
                                fc.emit(Op::CellSet(reg, value_reg));
                            } else {
                                fc.emit(Op::Move(reg, value_reg));
                            }
                        } else {
                            return Err(format!("undefined variable '{}' in assignment", name));
                        }
                    }
                    TypedAssignTarget::Field(obj, field, _) => {
                        let obj_reg = self.compile_expr(fc, obj)?;
                        let field_idx = self.get_field_index(&obj.ty, field)?;
                        fc.emit(Op::SetField(obj_reg, field_idx, value_reg));
                    }
                    TypedAssignTarget::Index(obj, idx, _) => {
                        let obj_reg = self.compile_expr(fc, obj)?;
                        let idx_reg = self.compile_expr(fc, idx)?;
                        fc.emit(Op::SetIndex(obj_reg, idx_reg, value_reg));
                    }
                }
                Ok(None)
            }
            TypedStmt::Return(Some(expr)) => {
                let reg = self.compile_expr(fc, expr)?;
                fc.emit(Op::Return(reg));
                Ok(None)
            }
            TypedStmt::Return(None) => {
                fc.emit(Op::ReturnNil);
                Ok(None)
            }
            TypedStmt::ReturnError(expr) => {
                let reg = self.compile_expr(fc, expr)?;
                let err_reg = fc.alloc_reg();
                fc.emit(Op::MakeError(err_reg, reg));
                fc.emit(Op::Return(err_reg));
                Ok(None)
            }
            TypedStmt::Break => {
                let jump_idx = fc.emit(Op::Jump(0)); // placeholder
                if let Some(breaks) = fc.loop_breaks.last_mut() {
                    breaks.push(jump_idx);
                }
                Ok(None)
            }
            TypedStmt::Continue => {
                let jump_idx = fc.emit(Op::Jump(0)); // placeholder
                if let Some(continues) = fc.loop_continues.last_mut() {
                    continues.push(jump_idx);
                }
                Ok(None)
            }
            TypedStmt::Expr(expr) => {
                let reg = self.compile_expr(fc, expr)?;
                Ok(Some(reg))
            }
        }
    }

    fn compile_expr(&mut self, fc: &mut FnCompiler, expr: &TypedExpr) -> Result<u16, String> {
        fc.current_line = expr.span.line;
        match &expr.kind {
            TypedExprKind::IntLit(n) => {
                let reg = fc.alloc_reg();
                fc.emit(Op::LoadInt(reg, *n));
                Ok(reg)
            }
            TypedExprKind::FloatLit(f) => {
                let reg = fc.alloc_reg();
                fc.emit(Op::LoadFloat(reg, *f));
                Ok(reg)
            }
            TypedExprKind::BoolLit(b) => {
                let reg = fc.alloc_reg();
                fc.emit(Op::LoadBool(reg, *b));
                Ok(reg)
            }
            TypedExprKind::StringLit(s) => {
                let reg = fc.alloc_reg();
                let idx = self.intern_string(s);
                fc.emit(Op::LoadString(reg, idx));
                Ok(reg)
            }
            TypedExprKind::InterpolatedString(parts) => self.compile_interpolated_string(fc, parts),
            TypedExprKind::Nil => {
                let reg = fc.alloc_reg();
                fc.emit(Op::LoadNil(reg));
                Ok(reg)
            }
            TypedExprKind::Ident(name) => {
                if let Some(&reg) = fc.locals.get(name) {
                    let dst = fc.alloc_reg();
                    if fc.captured_vars.contains(name) {
                        // Read through capture cell
                        fc.emit(Op::CellGet(dst, reg));
                    } else {
                        fc.emit(Op::Move(dst, reg));
                    }
                    Ok(dst)
                } else if let Some(idx) = self.functions.iter().position(|f| f.name == *name) {
                    // Function reference (by index)
                    let reg = fc.alloc_reg();
                    fc.emit(Op::LoadInt(reg, idx as i64));
                    Ok(reg)
                } else {
                    Err(format!("undefined variable '{}'", name))
                }
            }
            TypedExprKind::BinOp(left, op, right) => {
                self.compile_binop(fc, left, op, right, &expr.ty)
            }
            TypedExprKind::UnaryOp(op, operand) => {
                let src = self.compile_expr(fc, operand)?;
                let dst = fc.alloc_reg();
                match op {
                    UnaryOp::Neg => {
                        if operand.ty == Type::F64 {
                            fc.emit(Op::NegFloat(dst, src));
                        } else {
                            fc.emit(Op::NegInt(dst, src));
                            Self::emit_narrow_check(fc, dst, &expr.ty);
                        }
                    }
                    UnaryOp::Not => {
                        fc.emit(Op::Not(dst, src));
                    }
                    UnaryOp::BitNot => {
                        fc.emit(Op::BitNot(dst, src));
                        Self::emit_narrow_check(fc, dst, &expr.ty);
                    }
                }
                Ok(dst)
            }
            TypedExprKind::FieldAccess(obj, field) => {
                let obj_reg = self.compile_expr(fc, obj)?;
                let dst = fc.alloc_reg();

                // Handle .length
                if field == "length" {
                    match &obj.ty {
                        Type::String => {
                            fc.emit(Op::StringLen(dst, obj_reg));
                        }
                        Type::Array(_) => {
                            fc.emit(Op::ArrayLen(dst, obj_reg));
                        }
                        Type::Map(_, _) => {
                            fc.emit(Op::MapLen(dst, obj_reg));
                        }
                        Type::Tuple(elems) => {
                            fc.emit(Op::LoadInt(dst, elems.len() as i64));
                        }
                        _ => return Err(format!("no .length on {}", obj.ty.display_name())),
                    }
                    return Ok(dst);
                }

                // Handle tuple index
                if let Type::Tuple(_) = &obj.ty {
                    if let Ok(idx) = field.parse::<u16>() {
                        fc.emit(Op::GetField(dst, obj_reg, idx));
                        return Ok(dst);
                    }
                }

                // Struct field
                let field_idx = self.get_field_index(&obj.ty, field)?;
                fc.emit(Op::GetField(dst, obj_reg, field_idx));
                Ok(dst)
            }
            TypedExprKind::Index(obj, idx) => {
                let obj_reg = self.compile_expr(fc, obj)?;
                let idx_reg = self.compile_expr(fc, idx)?;
                let dst = fc.alloc_reg();
                fc.emit(Op::GetIndex(dst, obj_reg, idx_reg));
                Ok(dst)
            }
            TypedExprKind::Call(func, args) => {
                let func_reg = self.compile_expr(fc, func)?;
                let mut arg_regs = Vec::new();
                for arg in args {
                    arg_regs.push(self.compile_expr(fc, arg)?);
                }
                // Allocate a fresh contiguous call frame to avoid clobbering
                let frame_func = fc.alloc_reg();
                fc.emit(Op::Move(frame_func, func_reg));
                for &ar in &arg_regs {
                    let slot = fc.alloc_reg();
                    if ar != slot {
                        fc.emit(Op::Move(slot, ar));
                    }
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::Call(dst, frame_func, args.len() as u8));
                Ok(dst)
            }
            TypedExprKind::MethodCall(obj, method, args) => {
                let obj_reg = self.compile_expr(fc, obj)?;
                let method_id = self.intern_method(method);

                let mut arg_regs = Vec::new();
                for arg in args {
                    arg_regs.push(self.compile_expr(fc, arg)?);
                }
                // Allocate a fresh contiguous frame: [obj, arg0, arg1, ...]
                let frame_obj = fc.alloc_reg();
                fc.emit(Op::Move(frame_obj, obj_reg));
                for &ar in &arg_regs {
                    let slot = fc.alloc_reg();
                    if ar != slot {
                        fc.emit(Op::Move(slot, ar));
                    }
                }

                let dst = fc.alloc_reg();

                // Check if it's a capability method
                match &obj.ty {
                    Type::Capability(_) => {
                        fc.emit(Op::CallCapability(
                            dst,
                            frame_obj,
                            method_id,
                            args.len() as u8,
                        ));
                    }
                    _ => {
                        fc.emit(Op::CallMethod(dst, frame_obj, method_id, args.len() as u8));
                    }
                }
                Ok(dst)
            }
            TypedExprKind::StaticMethodCall(type_name, method, args) => {
                // Check if it's a module.function call
                let mut arg_regs = Vec::new();
                for arg in args {
                    let r = self.compile_expr(fc, arg)?;
                    arg_regs.push(r);
                }
                // Allocate a fresh contiguous frame for args
                let frame_start = fc.alloc_reg(); // placeholder for obj/func slot
                fc.emit(Op::LoadNil(frame_start));
                for &ar in &arg_regs {
                    let slot = fc.alloc_reg();
                    if ar != slot {
                        fc.emit(Op::Move(slot, ar));
                    }
                }

                let dst = fc.alloc_reg();
                let method_id = self.intern_method(&format!("{}.{}", type_name, method));
                fc.emit(Op::CallMethod(dst, frame_start, method_id, args.len() as u8));
                Ok(dst)
            }
            TypedExprKind::ArrayLit(elems) => {
                let mut elem_regs = Vec::new();
                for elem in elems {
                    elem_regs.push(self.compile_expr(fc, elem)?);
                }
                // Move to consecutive registers
                let start_reg = fc.registers;
                for &er in &elem_regs {
                    let slot = fc.alloc_reg();
                    if er != slot {
                        fc.emit(Op::Move(slot, er));
                    }
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeArray(dst, start_reg, elems.len() as u16));
                Ok(dst)
            }
            TypedExprKind::MapLit(entries) => {
                let mut kv_regs = Vec::new();
                for (key, val) in entries {
                    kv_regs.push(self.compile_expr(fc, key)?);
                    kv_regs.push(self.compile_expr(fc, val)?);
                }
                // Move to consecutive registers (key0, val0, key1, val1, ...)
                let start_reg = fc.registers;
                for &r in &kv_regs {
                    let slot = fc.alloc_reg();
                    if r != slot {
                        fc.emit(Op::Move(slot, r));
                    }
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeMap(dst, start_reg, entries.len() as u16));
                Ok(dst)
            }
            TypedExprKind::TupleLit(elems) => {
                let mut elem_regs = Vec::new();
                for elem in elems {
                    elem_regs.push(self.compile_expr(fc, elem)?);
                }
                // Move to consecutive registers
                let start_reg = fc.registers;
                for &er in &elem_regs {
                    let slot = fc.alloc_reg();
                    if er != slot {
                        fc.emit(Op::Move(slot, er));
                    }
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeTuple(dst, start_reg, elems.len() as u16));
                Ok(dst)
            }
            TypedExprKind::StructLit(type_name, fields) => {
                let type_id = self.intern_type(type_name);
                // Compile field expressions first, collecting result registers
                let field_order = self.get_field_order(type_name);
                let mut field_regs = Vec::new();
                for field_name in &field_order {
                    if let Some((_, expr)) = fields.iter().find(|(n, _)| n == field_name) {
                        field_regs.push(self.compile_expr(fc, expr)?);
                    }
                }
                // Move to consecutive registers right before dst
                for &fr in &field_regs {
                    let slot = fc.alloc_reg();
                    if fr != slot {
                        fc.emit(Op::Move(slot, fr));
                    }
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeStruct(dst, type_id, fields.len() as u8));
                Ok(dst)
            }
            TypedExprKind::EnumVariant(type_name, variant, args) => {
                let type_id = self.intern_type(type_name);
                let variant_idx = self.get_variant_index(type_name, variant)?;
                // Compile args, then move to consecutive registers before dst
                let mut arg_regs = Vec::new();
                for arg in args {
                    arg_regs.push(self.compile_expr(fc, arg)?);
                }
                for &ar in &arg_regs {
                    let slot = fc.alloc_reg();
                    if ar != slot {
                        fc.emit(Op::Move(slot, ar));
                    }
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeEnum(dst, type_id, variant_idx, args.len() as u8));
                Ok(dst)
            }
            TypedExprKind::ErrorLit(inner) => {
                let val_reg = self.compile_expr(fc, inner)?;
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeError(dst, val_reg));
                Ok(dst)
            }
            TypedExprKind::If(cond, then_block, else_expr) => {
                let cond_reg = self.compile_expr(fc, cond)?;
                let jump_false = fc.emit(Op::JumpIfFalse(cond_reg, 0));

                let then_result = self.compile_block(fc, then_block)?;
                let result_reg = if let Some(r) = then_result {
                    r
                } else {
                    let r = fc.alloc_reg();
                    fc.emit(Op::LoadNil(r));
                    r
                };

                if let Some(else_expr) = else_expr {
                    let jump_end = fc.emit(Op::Jump(0));
                    let else_start = fc.current_offset();
                    fc.patch_jump(jump_false, else_start);

                    let else_reg = self.compile_expr(fc, else_expr)?;
                    fc.emit(Op::Move(result_reg, else_reg));

                    let end = fc.current_offset();
                    fc.patch_jump(jump_end, end);
                } else {
                    let end = fc.current_offset();
                    fc.patch_jump(jump_false, end);
                }

                Ok(result_reg)
            }
            TypedExprKind::Match(scrutinee, arms) => {
                self.compile_match(fc, scrutinee, arms, &expr.ty)
            }
            TypedExprKind::For(var1, var2, iterable, body) => {
                self.compile_for(fc, var1, var2.as_deref(), iterable, body)
            }
            TypedExprKind::While(cond, body) => {
                let loop_start = fc.current_offset();

                fc.loop_breaks.push(Vec::new());
                fc.loop_continues.push(Vec::new());
                fc.loop_starts.push(loop_start);

                let cond_reg = self.compile_expr(fc, cond)?;
                let exit_jump = fc.emit(Op::JumpIfFalse(cond_reg, 0));

                self.compile_block(fc, body)?;

                // Jump back to condition
                let back_offset = loop_start as i32 - fc.current_offset() as i32;
                fc.emit(Op::Jump(back_offset));

                let loop_end = fc.current_offset();
                fc.patch_jump(exit_jump, loop_end);

                // Patch breaks
                let breaks = fc.loop_breaks.pop().unwrap_or_default();
                for b in breaks {
                    fc.patch_jump(b, loop_end);
                }
                // Patch continues
                let continues = fc.loop_continues.pop().unwrap_or_default();
                for c in continues {
                    fc.patch_jump(c, loop_start);
                }
                fc.loop_starts.pop();

                let dst = fc.alloc_reg();
                fc.emit(Op::LoadNil(dst));
                Ok(dst)
            }
            TypedExprKind::Loop(body) => {
                let loop_start = fc.current_offset();

                fc.loop_breaks.push(Vec::new());
                fc.loop_continues.push(Vec::new());
                fc.loop_starts.push(loop_start);

                self.compile_block(fc, body)?;

                let back_offset = loop_start as i32 - fc.current_offset() as i32;
                fc.emit(Op::Jump(back_offset));

                let loop_end = fc.current_offset();
                let breaks = fc.loop_breaks.pop().unwrap_or_default();
                for b in breaks {
                    fc.patch_jump(b, loop_end);
                }
                let continues = fc.loop_continues.pop().unwrap_or_default();
                for c in continues {
                    fc.patch_jump(c, loop_start);
                }
                fc.loop_starts.pop();

                let dst = fc.alloc_reg();
                fc.emit(Op::LoadNil(dst));
                Ok(dst)
            }
            TypedExprKind::Guard(binding, expr, else_block) => {
                if let Some(name) = binding {
                    // guard let name = expr else { ... }
                    let val_reg = self.compile_expr(fc, expr)?;

                    // Check if nil or error
                    let is_err = fc.alloc_reg();
                    fc.emit(Op::IsError(is_err, val_reg));
                    let jump_err = fc.emit(Op::JumpIfTrue(is_err, 0));

                    let is_nil = fc.alloc_reg();
                    fc.emit(Op::IsNil(is_nil, val_reg));
                    let jump_nil = fc.emit(Op::JumpIfTrue(is_nil, 0));

                    // Success path: bind the value
                    let local_reg = fc.declare_local(name, &expr.ty);
                    fc.emit(Op::Move(local_reg, val_reg));
                    let jump_past = fc.emit(Op::Jump(0));

                    // Failure path
                    let fail_start = fc.current_offset();
                    fc.patch_jump(jump_err, fail_start);
                    fc.patch_jump(jump_nil, fail_start);
                    self.compile_block(fc, else_block)?;

                    let end = fc.current_offset();
                    fc.patch_jump(jump_past, end);
                } else {
                    // guard condition else { ... }
                    let cond_reg = self.compile_expr(fc, expr)?;
                    let jump_false = fc.emit(Op::JumpIfFalse(cond_reg, 0));
                    let jump_past = fc.emit(Op::Jump(0));

                    let else_start = fc.current_offset();
                    fc.patch_jump(jump_false, else_start);
                    self.compile_block(fc, else_block)?;

                    let end = fc.current_offset();
                    fc.patch_jump(jump_past, end);
                }

                let dst = fc.alloc_reg();
                fc.emit(Op::LoadNil(dst));
                Ok(dst)
            }
            TypedExprKind::Block(block) => {
                let result = self.compile_block(fc, block)?;
                Ok(result.unwrap_or_else(|| {
                    let r = fc.alloc_reg();
                    fc.emit(Op::LoadNil(r));
                    r
                }))
            }
            TypedExprKind::Lambda(params, ret_type, body) => {
                // Capture analysis: find free variables used in the lambda body
                let free_vars = collect_free_vars(body, params);

                // Filter to only variables that exist in the current scope
                // (not global functions which are resolved by name)
                let captures: Vec<(String, Type)> = free_vars
                    .iter()
                    .filter_map(|name| {
                        fc.locals.get(name).and_then(|_| {
                            fc.local_types.get(name).map(|ty| (name.clone(), ty.clone()))
                        })
                    })
                    .collect();

                // Build the lambda's parameter list: captures first, then declared params
                let mut all_params = Vec::new();
                for (name, ty) in &captures {
                    all_params.push((name.clone(), ty.clone()));
                }
                all_params.extend(params.clone());

                // Determine which captures are cells (from the enclosing scope)
                let cell_param_names: Vec<String> = captures
                    .iter()
                    .filter(|(name, _)| fc.captured_vars.contains(name))
                    .map(|(name, _)| name.clone())
                    .collect();

                let func_name = format!("__lambda_{}", self.functions.len());
                let typed_fndef = TypedFnDef {
                    name: func_name.clone(),
                    params: all_params,
                    return_type: ret_type.clone(),
                    body: body.clone(),
                };
                let func_idx = self.compile_fn(&typed_fndef, &cell_param_names)?;

                if captures.is_empty() {
                    // No captures — simple function reference
                    let dst = fc.alloc_reg();
                    fc.emit(Op::LoadInt(dst, func_idx as i64));
                    Ok(dst)
                } else {
                    // Emit captured values into consecutive registers, then MakeClosure
                    let func_reg = fc.alloc_reg();
                    fc.emit(Op::LoadInt(func_reg, func_idx as i64));
                    for (cap_name, _) in &captures {
                        // SAFETY: captures come from type-checker analysis; variables must exist in enclosing scope
                        let src = *fc.locals.get(cap_name).unwrap();
                        let slot = fc.alloc_reg();
                        fc.emit(Op::Move(slot, src));
                    }
                    let dst = fc.alloc_reg();
                    fc.emit(Op::MakeClosure(dst, func_reg, captures.len() as u16));
                    Ok(dst)
                }
            }
            TypedExprKind::As(inner, target_ty) => {
                let src = self.compile_expr(fc, inner)?;
                let dst = fc.alloc_reg();
                self.emit_conversion(fc, dst, src, &inner.ty, target_ty, false)?;
                Ok(dst)
            }
            TypedExprKind::AsSafe(inner, target_ty) => {
                let src = self.compile_expr(fc, inner)?;
                let dst = fc.alloc_reg();
                self.emit_conversion(fc, dst, src, &inner.ty, target_ty, true)?;
                Ok(dst)
            }
            TypedExprKind::Is(inner, target) => {
                let src = self.compile_expr(fc, inner)?;
                let dst = fc.alloc_reg();
                match target {
                    TypedIsTarget::Type(ty_expr) => {
                        let type_name = canonical_type_name(ty_expr);
                        let type_id = self.intern_type(&type_name);
                        fc.emit(Op::IsType(dst, src, type_id));
                    }
                    TypedIsTarget::EnumVariant(type_name, variant) => {
                        let variant_idx = self.get_variant_index(type_name, variant).unwrap_or(0);
                        fc.emit(Op::IsEnumVariant(dst, src, variant_idx));
                    }
                    TypedIsTarget::QualifiedVariant(module, type_name, variant) => {
                        let full_name = format!("{}.{}", module, type_name);
                        let variant_idx = self.get_variant_index(&full_name, variant).unwrap_or(0);
                        fc.emit(Op::IsEnumVariant(dst, src, variant_idx));
                    }
                    TypedIsTarget::Expr(rhs_expr) => {
                        let rhs = self.compile_expr(fc, rhs_expr)?;
                        fc.emit(Op::IsIdentical(dst, src, rhs));
                    }
                }
                Ok(dst)
            }
            TypedExprKind::Try(inner) => {
                let val_reg = self.compile_expr(fc, inner)?;
                let is_err = fc.alloc_reg();
                fc.emit(Op::IsError(is_err, val_reg));
                let jump = fc.emit(Op::JumpIfFalse(is_err, 0));
                // Error path: propagate
                fc.emit(Op::Return(val_reg));
                // Success path
                let end = fc.current_offset();
                fc.patch_jump(jump, end);
                // Unwrap the success value
                Ok(val_reg)
            }
            TypedExprKind::Range(start, end) => {
                let start_reg = self.compile_expr(fc, start)?;
                let end_reg = self.compile_expr(fc, end)?;
                let dst = fc.alloc_reg();
                fc.emit(Op::MakeRange(dst, start_reg, end_reg));
                Ok(dst)
            }
            TypedExprKind::Panic(args) => {
                if let Some(arg) = args.first() {
                    let reg = self.compile_expr(fc, arg)?;
                    fc.emit(Op::Panic(reg));
                } else {
                    let reg = fc.alloc_reg();
                    let idx = self.intern_string("panic");
                    fc.emit(Op::LoadString(reg, idx));
                    fc.emit(Op::Panic(reg));
                }
                let dst = fc.alloc_reg();
                fc.emit(Op::LoadNil(dst));
                Ok(dst)
            }
        }
    }

    fn compile_binop(
        &mut self,
        fc: &mut FnCompiler,
        left: &TypedExpr,
        op: &BinOp,
        right: &TypedExpr,
        result_ty: &Type,
    ) -> Result<u16, String> {
        // Short-circuit for && and ||
        match op {
            BinOp::And => {
                let left_reg = self.compile_expr(fc, left)?;
                let dst = fc.alloc_reg();
                fc.emit(Op::Move(dst, left_reg));
                let short_circuit = fc.emit(Op::JumpIfFalse(dst, 0));
                let right_reg = self.compile_expr(fc, right)?;
                fc.emit(Op::Move(dst, right_reg));
                let end = fc.current_offset();
                fc.patch_jump(short_circuit, end);
                return Ok(dst);
            }
            BinOp::Or => {
                let left_reg = self.compile_expr(fc, left)?;
                let dst = fc.alloc_reg();
                fc.emit(Op::Move(dst, left_reg));
                let short_circuit = fc.emit(Op::JumpIfTrue(dst, 0));
                let right_reg = self.compile_expr(fc, right)?;
                fc.emit(Op::Move(dst, right_reg));
                let end = fc.current_offset();
                fc.patch_jump(short_circuit, end);
                return Ok(dst);
            }
            _ => {}
        }

        let left_reg = self.compile_expr(fc, left)?;
        let right_reg = self.compile_expr(fc, right)?;
        let dst = fc.alloc_reg();

        let is_float = left.ty == Type::F64;
        let is_string = left.ty == Type::String;

        match op {
            BinOp::Add if is_string => {
                fc.emit(Op::ConcatString(dst, left_reg, right_reg));
            }
            BinOp::Add if is_float => {
                fc.emit(Op::AddFloat(dst, left_reg, right_reg));
            }
            BinOp::Add => {
                fc.emit(Op::AddInt(dst, left_reg, right_reg));
            }
            BinOp::Sub if is_float => {
                fc.emit(Op::SubFloat(dst, left_reg, right_reg));
            }
            BinOp::Sub => {
                fc.emit(Op::SubInt(dst, left_reg, right_reg));
            }
            BinOp::Mul if is_float => {
                fc.emit(Op::MulFloat(dst, left_reg, right_reg));
            }
            BinOp::Mul => {
                fc.emit(Op::MulInt(dst, left_reg, right_reg));
            }
            BinOp::Div if is_float => {
                fc.emit(Op::DivFloat(dst, left_reg, right_reg));
            }
            BinOp::Div => {
                fc.emit(Op::DivInt(dst, left_reg, right_reg));
            }
            BinOp::Mod if is_float => {
                fc.emit(Op::ModFloat(dst, left_reg, right_reg));
            }
            BinOp::Mod => {
                fc.emit(Op::ModInt(dst, left_reg, right_reg));
            }
            BinOp::Pow if is_float => {
                fc.emit(Op::PowFloat(dst, left_reg, right_reg));
            }
            BinOp::Pow => {
                fc.emit(Op::PowInt(dst, left_reg, right_reg));
            }
            BinOp::Eq if is_float => {
                fc.emit(Op::EqFloat(dst, left_reg, right_reg));
            }
            BinOp::Eq if is_string => {
                fc.emit(Op::EqString(dst, left_reg, right_reg));
            }
            BinOp::Eq if left.ty == Type::Bool => {
                fc.emit(Op::EqBool(dst, left_reg, right_reg));
            }
            BinOp::Eq => {
                fc.emit(Op::EqGeneric(dst, left_reg, right_reg));
            }
            BinOp::NotEq if is_float => {
                fc.emit(Op::NeqFloat(dst, left_reg, right_reg));
            }
            BinOp::NotEq if is_string => {
                fc.emit(Op::NeqString(dst, left_reg, right_reg));
            }
            BinOp::NotEq if left.ty == Type::Bool => {
                fc.emit(Op::NeqBool(dst, left_reg, right_reg));
            }
            BinOp::NotEq => {
                fc.emit(Op::NeqGeneric(dst, left_reg, right_reg));
            }
            BinOp::Lt if is_float => {
                fc.emit(Op::LtFloat(dst, left_reg, right_reg));
            }
            BinOp::Lt if is_string => {
                fc.emit(Op::LtString(dst, left_reg, right_reg));
            }
            BinOp::Lt => {
                fc.emit(Op::LtInt(dst, left_reg, right_reg));
            }
            BinOp::LtEq if is_float => {
                fc.emit(Op::LteFloat(dst, left_reg, right_reg));
            }
            BinOp::LtEq if is_string => {
                fc.emit(Op::LteString(dst, left_reg, right_reg));
            }
            BinOp::LtEq => {
                fc.emit(Op::LteInt(dst, left_reg, right_reg));
            }
            BinOp::Gt if is_float => {
                fc.emit(Op::GtFloat(dst, left_reg, right_reg));
            }
            BinOp::Gt if is_string => {
                fc.emit(Op::GtString(dst, left_reg, right_reg));
            }
            BinOp::Gt => {
                fc.emit(Op::GtInt(dst, left_reg, right_reg));
            }
            BinOp::GtEq if is_float => {
                fc.emit(Op::GteFloat(dst, left_reg, right_reg));
            }
            BinOp::GtEq if is_string => {
                fc.emit(Op::GteString(dst, left_reg, right_reg));
            }
            BinOp::GtEq => {
                fc.emit(Op::GteInt(dst, left_reg, right_reg));
            }
            BinOp::BitAnd => {
                fc.emit(Op::BitAnd(dst, left_reg, right_reg));
            }
            BinOp::BitOr => {
                fc.emit(Op::BitOr(dst, left_reg, right_reg));
            }
            BinOp::BitXor => {
                fc.emit(Op::BitXor(dst, left_reg, right_reg));
            }
            BinOp::ShiftLeft => {
                fc.emit(Op::ShiftLeft(dst, left_reg, right_reg));
            }
            BinOp::ShiftRight => {
                fc.emit(Op::ShiftRight(dst, left_reg, right_reg));
            }
            BinOp::And | BinOp::Or => unreachable!("handled above"),
        };

        // Narrow integer range check: if the result type is a narrow integer,
        // the spec requires overflow to panic. The i64 arithmetic may have
        // produced a value outside the narrow type's range.
        Self::emit_narrow_check(fc, dst, result_ty);

        Ok(dst)
    }

    /// If `ty` is a narrow integer type (not i64), emit IntNarrow to enforce
    /// the type's range. This is a no-op for i64, f64, bool, etc.
    fn emit_narrow_check(fc: &mut FnCompiler, reg: u16, ty: &Type) {
        let int_ty = match ty {
            Type::I8 => IntType::I8,
            Type::I16 => IntType::I16,
            Type::I32 => IntType::I32,
            Type::U8 => IntType::U8,
            Type::U16 => IntType::U16,
            Type::U32 => IntType::U32,
            Type::U64 => IntType::U64,
            _ => return, // i64, f64, bool, etc. — no narrowing needed
        };
        fc.emit(Op::IntNarrow(reg, reg, int_ty));
    }

    fn compile_match(
        &mut self,
        fc: &mut FnCompiler,
        scrutinee: &TypedExpr,
        arms: &[TypedMatchArm],
        _result_ty: &Type,
    ) -> Result<u16, String> {
        let scrutinee_reg = self.compile_expr(fc, scrutinee)?;
        let result_reg = fc.alloc_reg();
        fc.emit(Op::LoadNil(result_reg));

        let mut end_jumps = Vec::new();

        for arm in arms {
            let next_arm_jump =
                self.compile_pattern_test(fc, scrutinee_reg, &arm.pattern, &scrutinee.ty)?;

            // Bind variables
            fc.push_scope();
            self.compile_pattern_bindings(fc, scrutinee_reg, &arm.pattern, &scrutinee.ty)?;

            let body_reg = self.compile_expr(fc, &arm.body)?;
            fc.emit(Op::Move(result_reg, body_reg));
            fc.pop_scope();

            end_jumps.push(fc.emit(Op::Jump(0)));

            // Patch the "no match" jump to here
            if let Some(idx) = next_arm_jump {
                let here = fc.current_offset();
                fc.patch_jump(idx, here);
            }
        }

        let end = fc.current_offset();
        for j in end_jumps {
            fc.patch_jump(j, end);
        }

        Ok(result_reg)
    }

    fn compile_pattern_test(
        &mut self,
        fc: &mut FnCompiler,
        scrutinee_reg: u16,
        pattern: &Pattern,
        _scrutinee_ty: &Type,
    ) -> Result<Option<usize>, String> {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Binding(_) => {
                // Always matches
                Ok(None)
            }
            PatternKind::IntLit(n) => {
                let cmp_reg = fc.alloc_reg();
                let val_reg = fc.alloc_reg();
                fc.emit(Op::LoadInt(val_reg, *n));
                fc.emit(Op::EqInt(cmp_reg, scrutinee_reg, val_reg));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::FloatLit(f) => {
                let cmp_reg = fc.alloc_reg();
                let val_reg = fc.alloc_reg();
                fc.emit(Op::LoadFloat(val_reg, *f));
                fc.emit(Op::EqFloat(cmp_reg, scrutinee_reg, val_reg));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::BoolLit(b) => {
                let cmp_reg = fc.alloc_reg();
                let val_reg = fc.alloc_reg();
                fc.emit(Op::LoadBool(val_reg, *b));
                fc.emit(Op::EqBool(cmp_reg, scrutinee_reg, val_reg));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::StringLit(s) => {
                let cmp_reg = fc.alloc_reg();
                let val_reg = fc.alloc_reg();
                let idx = self.intern_string(s);
                fc.emit(Op::LoadString(val_reg, idx));
                fc.emit(Op::EqString(cmp_reg, scrutinee_reg, val_reg));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::Nil => {
                let cmp_reg = fc.alloc_reg();
                fc.emit(Op::IsNil(cmp_reg, scrutinee_reg));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::EnumVariant(type_name, variant, _)
            | PatternKind::QualifiedEnumVariant(_, type_name, variant, _) => {
                let full_name = match &pattern.kind {
                    PatternKind::QualifiedEnumVariant(m, t, _, _) => format!("{}.{}", m, t),
                    _ => type_name.clone(),
                };
                let variant_idx = self.get_variant_index(&full_name, variant)?;
                let cmp_reg = fc.alloc_reg();
                fc.emit(Op::IsEnumVariant(cmp_reg, scrutinee_reg, variant_idx));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::Error(_name) => {
                let is_err = fc.alloc_reg();
                fc.emit(Op::IsError(is_err, scrutinee_reg));
                let jump = fc.emit(Op::JumpIfFalse(is_err, 0));
                Ok(Some(jump))
            }
            PatternKind::IsType(ty_expr) => {
                let type_name = canonical_type_name(ty_expr);
                let type_id = self.intern_type(&type_name);
                let cmp_reg = fc.alloc_reg();
                fc.emit(Op::IsType(cmp_reg, scrutinee_reg, type_id));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
            PatternKind::IsEnumVariant(type_name, variant) => {
                let variant_idx = self.get_variant_index(type_name, variant)?;
                let cmp_reg = fc.alloc_reg();
                fc.emit(Op::IsEnumVariant(cmp_reg, scrutinee_reg, variant_idx));
                let jump = fc.emit(Op::JumpIfFalse(cmp_reg, 0));
                Ok(Some(jump))
            }
        }
    }

    fn compile_pattern_bindings(
        &mut self,
        fc: &mut FnCompiler,
        scrutinee_reg: u16,
        pattern: &Pattern,
        scrutinee_ty: &Type,
    ) -> Result<(), String> {
        match &pattern.kind {
            PatternKind::Binding(name) if name != "_" => {
                let reg = fc.declare_local(name, scrutinee_ty);
                fc.emit(Op::Move(reg, scrutinee_reg));
            }
            PatternKind::EnumVariant(_, _, bindings)
            | PatternKind::QualifiedEnumVariant(_, _, _, bindings) => {
                for (i, name) in bindings.iter().enumerate() {
                    if name != "_" {
                        let reg = fc.declare_local(name, &Type::Nil); // type resolved during type check
                        fc.emit(Op::GetEnumField(reg, scrutinee_reg, i as u8));
                    }
                }
            }
            PatternKind::Error(name) => {
                let reg = fc.declare_local(name, &Type::String);
                fc.emit(Op::UnwrapError(reg, scrutinee_reg));
            }
            _ => {}
        }
        Ok(())
    }

    fn compile_for(
        &mut self,
        fc: &mut FnCompiler,
        var1: &str,
        var2: Option<&str>,
        iterable: &TypedExpr,
        body: &TypedBlock,
    ) -> Result<u16, String> {
        let iter_src = self.compile_expr(fc, iterable)?;

        // Initialize iterator
        let iter_reg = fc.alloc_reg();
        fc.emit(Op::IterInit(iter_reg, iter_src));

        let loop_start = fc.current_offset();
        fc.loop_breaks.push(Vec::new());
        fc.loop_continues.push(Vec::new());
        fc.loop_starts.push(loop_start);

        fc.push_scope();

        let done_reg = fc.alloc_reg();

        if let Some(var2_name) = var2 {
            let v1_captured = fc.captured_vars.contains(var1);
            let v2_captured = fc.captured_vars.contains(var2_name);
            let key_reg = fc.declare_local(var1, &Type::Nil);
            let val_reg = fc.declare_local(var2_name, &Type::Nil);
            if v1_captured || v2_captured {
                // Use temp registers for IterNext, then write through cells
                let tmp_k = fc.alloc_reg();
                let tmp_v = fc.alloc_reg();
                fc.emit(Op::IterNextKV(tmp_k, tmp_v, done_reg, iter_reg));
                if v1_captured {
                    fc.emit(Op::MakeCell(key_reg, tmp_k));
                } else {
                    fc.emit(Op::Move(key_reg, tmp_k));
                }
                if v2_captured {
                    fc.emit(Op::MakeCell(val_reg, tmp_v));
                } else {
                    fc.emit(Op::Move(val_reg, tmp_v));
                }
            } else {
                fc.emit(Op::IterNextKV(key_reg, val_reg, done_reg, iter_reg));
            }
        } else {
            let v1_captured = fc.captured_vars.contains(var1);
            let val_reg = fc.declare_local(var1, &Type::Nil);
            if v1_captured {
                let tmp = fc.alloc_reg();
                fc.emit(Op::IterNext(tmp, done_reg, iter_reg));
                fc.emit(Op::MakeCell(val_reg, tmp));
            } else {
                fc.emit(Op::IterNext(val_reg, done_reg, iter_reg));
            }
        }

        let exit_jump = fc.emit(Op::JumpIfTrue(done_reg, 0));

        // Compile body
        self.compile_block(fc, body)?;

        fc.pop_scope();

        // Jump back to loop start
        let back_offset = loop_start as i32 - fc.current_offset() as i32;
        fc.emit(Op::Jump(back_offset));

        let loop_end = fc.current_offset();
        fc.patch_jump(exit_jump, loop_end);

        let breaks = fc.loop_breaks.pop().unwrap_or_default();
        for b in breaks {
            fc.patch_jump(b, loop_end);
        }
        let continues = fc.loop_continues.pop().unwrap_or_default();
        for c in continues {
            fc.patch_jump(c, loop_start);
        }
        fc.loop_starts.pop();

        let dst = fc.alloc_reg();
        fc.emit(Op::LoadNil(dst));
        Ok(dst)
    }

    fn compile_interpolated_string(
        &mut self,
        fc: &mut FnCompiler,
        parts: &[TypedStringPart],
    ) -> Result<u16, String> {
        if parts.is_empty() {
            let reg = fc.alloc_reg();
            let idx = self.intern_string("");
            fc.emit(Op::LoadString(reg, idx));
            return Ok(reg);
        }

        let mut result_reg = None;

        for part in parts {
            let part_reg = match part {
                TypedStringPart::Literal(s) => {
                    let reg = fc.alloc_reg();
                    let idx = self.intern_string(s);
                    fc.emit(Op::LoadString(reg, idx));
                    reg
                }
                TypedStringPart::Expr(expr) => {
                    let reg = self.compile_expr(fc, expr)?;
                    // Convert to string if needed
                    match &expr.ty {
                        Type::String => reg,
                        Type::I64
                        | Type::I8
                        | Type::I16
                        | Type::I32
                        | Type::U8
                        | Type::U16
                        | Type::U32
                        | Type::U64 => {
                            let str_reg = fc.alloc_reg();
                            fc.emit(Op::IntToString(str_reg, reg));
                            str_reg
                        }
                        Type::F64 => {
                            let str_reg = fc.alloc_reg();
                            fc.emit(Op::FloatToString(str_reg, reg));
                            str_reg
                        }
                        Type::Bool => {
                            let str_reg = fc.alloc_reg();
                            fc.emit(Op::BoolToString(str_reg, reg));
                            str_reg
                        }
                        Type::Nil => {
                            let str_reg = fc.alloc_reg();
                            let idx = self.intern_string("nil");
                            fc.emit(Op::LoadString(str_reg, idx));
                            str_reg
                        }
                        _ => {
                            // Generic display for structs, enums, etc.
                            let str_reg = fc.alloc_reg();
                            fc.emit(Op::DisplayToString(str_reg, reg));
                            str_reg
                        }
                    }
                }
            };

            result_reg = Some(if let Some(prev_reg) = result_reg {
                let concat_reg = fc.alloc_reg();
                fc.emit(Op::ConcatString(concat_reg, prev_reg, part_reg));
                concat_reg
            } else {
                part_reg
            });
        }

        // SAFETY: parts is non-empty (empty case returns early above), so loop runs at least once
        Ok(result_reg.unwrap())
    }

    fn emit_conversion(
        &mut self,
        fc: &mut FnCompiler,
        dst: u16,
        src: u16,
        from: &Type,
        to: &Type,
        safe: bool,
    ) -> Result<(), String> {
        match (from, to) {
            (f, t) if f.is_integer() && *t == Type::F64 => {
                fc.emit(Op::IntToFloat(dst, src));
            }
            (Type::F64, t) if t.is_integer() => {
                if safe {
                    fc.emit(Op::FloatToIntSafe(dst, src));
                } else {
                    fc.emit(Op::FloatToInt(dst, src));
                }
            }
            (f, Type::String) if f.is_integer() => {
                fc.emit(Op::IntToString(dst, src));
            }
            (Type::F64, Type::String) => {
                fc.emit(Op::FloatToString(dst, src));
            }
            (Type::Bool, Type::String) => {
                fc.emit(Op::BoolToString(dst, src));
            }
            (Type::Nil, Type::String) => {
                let idx = self.intern_string("nil");
                fc.emit(Op::LoadString(dst, idx));
            }
            (Type::String, t) if t.is_integer() => {
                if safe {
                    fc.emit(Op::StringToIntSafe(dst, src));
                } else {
                    fc.emit(Op::StringToInt(dst, src));
                }
            }
            (Type::String, Type::F64) => {
                if safe {
                    fc.emit(Op::StringToFloatSafe(dst, src));
                } else {
                    fc.emit(Op::StringToFloat(dst, src));
                }
            }
            (f, t) if f.is_integer() && t.is_integer() => {
                // Integer narrowing/widening
                if safe {
                    fc.emit(Op::IntNarrowSafe(dst, src, type_to_int_type(to)));
                } else {
                    fc.emit(Op::IntNarrow(dst, src, type_to_int_type(to)));
                }
            }
            (_, Type::String) => {
                // Generic display conversion for Optional, Enum, Struct, etc.
                fc.emit(Op::DisplayToString(dst, src));
            }
            _ => {
                fc.emit(Op::Move(dst, src));
            }
        }
        Ok(())
    }

    fn get_field_index(&self, ty: &Type, field: &str) -> Result<u16, String> {
        match ty {
            Type::Struct(name) => {
                if let Some(info) = self.type_info.structs.get(name) {
                    for (i, (fname, _)) in info.fields.iter().enumerate() {
                        if fname == field {
                            return Ok(i as u16);
                        }
                    }
                }
                // Check module structs
                if name.contains('.') {
                    let parts: Vec<&str> = name.splitn(2, '.').collect();
                    if let Some(mod_info) = self.type_info.modules.get(parts[0]) {
                        if let Some(struct_info) = mod_info.structs.get(parts[1]) {
                            for (i, (fname, _)) in struct_info.fields.iter().enumerate() {
                                if fname == field {
                                    return Ok(i as u16);
                                }
                            }
                        }
                    }
                }
                Err(format!("unknown field '{}' on struct '{}'", field, name))
            }
            _ => Err(format!("cannot get field index on {}", ty.display_name())),
        }
    }

    fn get_field_order(&self, type_name: &str) -> Vec<String> {
        if let Some(info) = self.type_info.structs.get(type_name) {
            return info.fields.iter().map(|(n, _)| n.clone()).collect();
        }
        // Check modules
        if type_name.contains('.') {
            let parts: Vec<&str> = type_name.splitn(2, '.').collect();
            if let Some(mod_info) = self.type_info.modules.get(parts[0]) {
                if let Some(info) = mod_info.structs.get(parts[1]) {
                    return info.fields.iter().map(|(n, _)| n.clone()).collect();
                }
            }
        }
        Vec::new()
    }

    fn get_variant_index(&self, type_name: &str, variant: &str) -> Result<u8, String> {
        if let Some(info) = self.type_info.enums.get(type_name) {
            for (i, v) in info.variants.iter().enumerate() {
                if v.name == variant {
                    return Ok(i as u8);
                }
            }
        }
        // Check modules
        if type_name.contains('.') {
            let parts: Vec<&str> = type_name.splitn(2, '.').collect();
            if let Some(mod_info) = self.type_info.modules.get(parts[0]) {
                if let Some(info) = mod_info.enums.get(parts[1]) {
                    for (i, v) in info.variants.iter().enumerate() {
                        if v.name == variant {
                            return Ok(i as u8);
                        }
                    }
                }
            }
        }
        Err(format!("unknown variant '{}.{}'", type_name, variant))
    }
}

/// Find all variables in a function body that are captured by any nested lambda.
fn canonical_type_name(ty: &crate::ast::TypeExpr) -> String {
    use crate::ast::TypeExpr;
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Qualified(m, n) => format!("{}.{}", m, n),
        TypeExpr::Array(inner) => format!("[{}]", canonical_type_name(inner)),
        TypeExpr::Map(k, v) => format!("[{}: {}]", canonical_type_name(k), canonical_type_name(v)),
        TypeExpr::Tuple(ts) => format!(
            "({})",
            ts.iter()
                .map(canonical_type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypeExpr::Optional(inner) => format!("{}?", canonical_type_name(inner)),
        TypeExpr::Result(ok, err) => {
            format!("{}!{}", canonical_type_name(ok), canonical_type_name(err))
        }
        TypeExpr::Function(params, ret) => format!(
            "fn({}) -> {}",
            params
                .iter()
                .map(canonical_type_name)
                .collect::<Vec<_>>()
                .join(", "),
            canonical_type_name(ret)
        ),
        TypeExpr::Union(ms) => ms
            .iter()
            .map(canonical_type_name)
            .collect::<Vec<_>>()
            .join(" | "),
        TypeExpr::SelfType => "Self".to_string(),
    }
}

fn type_to_int_type(ty: &Type) -> IntType {
    match ty {
        Type::I8 => IntType::I8,
        Type::I16 => IntType::I16,
        Type::I32 => IntType::I32,
        Type::I64 => IntType::I64,
        Type::U8 => IntType::U8,
        Type::U16 => IntType::U16,
        Type::U32 => IntType::U32,
        Type::U64 => IntType::U64,
        _ => IntType::I64,
    }
}
