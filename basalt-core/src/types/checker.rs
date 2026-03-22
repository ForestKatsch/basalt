use super::*;
use crate::error::{CompileError, CompileErrors};
use std::collections::{HashMap, HashSet};

/// Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate().take(n + 1) {
        *val = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a.as_bytes()[i - 1] == b.as_bytes()[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}

/// Find the closest matching name from candidates, within a length-dependent threshold.
fn suggest_similar<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let max_distance = match name.len() {
        0..=2 => 1,
        3..=5 => 2,
        _ => 3,
    };
    candidates
        .filter(|c| *c != name)
        .filter_map(|c| {
            let d = edit_distance(name, c);
            if d <= max_distance {
                Some((c, d))
            } else {
                None
            }
        })
        .min_by_key(|(_, d)| *d)
        .map(|(s, _)| s.to_string())
}

struct TypeChecker {
    type_info: TypeInfo,
    scopes: Vec<HashMap<String, (Type, bool)>>, // name -> (type, mutable)
    current_fn_return: Option<Type>,
    in_loop: bool,
    current_type_name: Option<String>,
    /// Variables that are both mutable and captured by a closure in the current
    /// function. Narrowing these is unsound because any function call could
    /// invoke the capturing closure and mutate the variable.
    captured_mut_vars: HashSet<String>,
}

pub fn check(program: &Program) -> Result<TypedProgram, CompileErrors> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}

impl TypeChecker {
    fn new() -> Self {
        TypeChecker {
            type_info: TypeInfo {
                structs: HashMap::new(),
                enums: HashMap::new(),
                aliases: HashMap::new(),
                functions: HashMap::new(),
                modules: HashMap::new(),
            },
            scopes: vec![HashMap::new()],
            current_fn_return: None,
            in_loop: false,
            current_type_name: None,
            captured_mut_vars: HashSet::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_var(&mut self, name: &str, ty: Type, mutable: bool) -> Result<(), CompileError> {
        // Reject names that collide with imported module names
        if self.type_info.modules.contains_key(name) {
            return Err(CompileError::bare(format!(
                "'{}' is already the name of an imported module",
                name
            )));
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (ty, mutable));
        }
        Ok(())
    }

    /// Remove a type from a union, returning the remaining type.
    /// If `original` is `A | B | C` and `to_remove` is `A`, returns `B | C`.
    /// If the result would be empty, returns the original unchanged.
    fn subtract_type(&self, original: &Type, to_remove: &Type) -> Type {
        match original {
            Type::Union(members) => {
                let remaining: Vec<Type> = members
                    .iter()
                    .filter(|m| m != &to_remove)
                    .cloned()
                    .collect();
                match remaining.len() {
                    0 => original.clone(),
                    // SAFETY: remaining.len() == 1 per match arm
                    1 => remaining.into_iter().next().unwrap(),
                    _ => Type::Union(remaining),
                }
            }
            Type::Optional(inner) if to_remove == &Type::Nil => *inner.clone(),
            _ => original.clone(),
        }
    }

    fn lookup_var(&self, name: &str) -> Option<(Type, bool)> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry.clone());
            }
        }
        None
    }

    /// Resolve a type expression, checking module-local types first.
    /// Used when resolving types inside a module definition.
    fn resolve_type_expr_in_module(
        &self,
        ty: &TypeExpr,
        module_name: &str,
    ) -> Result<Type, CompileError> {
        if let TypeExpr::Named(name) = ty {
            // Try qualified name first (module.Type)
            let qualified = format!("{}.{}", module_name, name);
            if self.type_info.structs.contains_key(&qualified) {
                return Ok(Type::Struct(qualified));
            }
            if self.type_info.enums.contains_key(&qualified) {
                return Ok(Type::Enum(qualified));
            }
        }
        // Fall back to normal resolution
        self.resolve_type_expr(ty)
    }

    fn resolve_type_expr(&self, ty: &TypeExpr) -> Result<Type, CompileError> {
        match ty {
            TypeExpr::Named(name) => self.resolve_type_name(name),
            TypeExpr::Qualified(module, name) => {
                if let Some(mod_info) = self.type_info.modules.get(module) {
                    if mod_info.structs.contains_key(name) {
                        return Ok(Type::Struct(format!("{}.{}", module, name)));
                    }
                    if mod_info.enums.contains_key(name) {
                        return Ok(Type::Enum(format!("{}.{}", module, name)));
                    }
                    if let Some(alias) = mod_info.aliases.get(name) {
                        return Ok(alias.clone());
                    }
                }
                Err(CompileError::bare(format!(
                    "unknown type {}.{}",
                    module, name
                )))
            }
            TypeExpr::Array(inner) => Ok(Type::Array(Box::new(self.resolve_type_expr(inner)?))),
            TypeExpr::Map(k, v) => Ok(Type::Map(
                Box::new(self.resolve_type_expr(k)?),
                Box::new(self.resolve_type_expr(v)?),
            )),
            TypeExpr::Tuple(types) => {
                let resolved: Result<Vec<_>, _> =
                    types.iter().map(|t| self.resolve_type_expr(t)).collect();
                Ok(Type::Tuple(resolved?))
            }
            TypeExpr::Optional(inner) => {
                let resolved = self.resolve_type_expr(inner)?;
                // Reject T?? — double optional is redundant (T? = T | nil, T?? = T | nil | nil = T | nil)
                if matches!(resolved, Type::Optional(_)) {
                    return Err(CompileError::bare(
                        "double optional T?? is not allowed; T? already includes nil",
                    ));
                }
                Ok(Type::Optional(Box::new(resolved)))
            }
            TypeExpr::Result(ok, err) => {
                let ok_ty = self.resolve_type_expr(ok)?;
                let err_ty = self.resolve_type_expr(err)?;
                // Reject T!E? and T?!E — ambiguous composite types
                if matches!(ok_ty, Type::Optional(_)) {
                    return Err(CompileError::bare(
                        "T?!E is not allowed; wrap in parentheses or restructure the type",
                    ));
                }
                if matches!(err_ty, Type::Optional(_)) {
                    return Err(CompileError::bare(
                        "T!E? is not allowed; wrap in parentheses or restructure the type",
                    ));
                }
                Ok(Type::Result(Box::new(ok_ty), Box::new(err_ty)))
            }
            TypeExpr::Function(params, ret) => {
                let p: Result<Vec<_>, _> =
                    params.iter().map(|t| self.resolve_type_expr(t)).collect();
                Ok(Type::Function(p?, Box::new(self.resolve_type_expr(ret)?)))
            }
            TypeExpr::Union(members) => {
                let resolved: Result<Vec<_>, _> =
                    members.iter().map(|t| self.resolve_type_expr(t)).collect();
                let mut flat = Vec::new();
                for ty in resolved? {
                    match ty {
                        Type::Union(inner) => flat.extend(inner),
                        other => flat.push(other),
                    }
                }
                // Deduplicate
                let mut deduped = Vec::new();
                for ty in flat {
                    if !deduped.contains(&ty) {
                        deduped.push(ty);
                    }
                }
                if deduped.len() == 1 {
                    // SAFETY: deduped is non-empty, guarded by len==1 check above
                    Ok(deduped.into_iter().next().unwrap())
                } else {
                    Ok(Type::Union(deduped))
                }
            }
            TypeExpr::SelfType => {
                if let Some(ref name) = self.current_type_name {
                    // Check if it's a struct or enum
                    if self.type_info.structs.contains_key(name) {
                        Ok(Type::Struct(name.clone()))
                    } else if self.type_info.enums.contains_key(name) {
                        Ok(Type::Enum(name.clone()))
                    } else {
                        Err(CompileError::bare("Self used outside of type definition"))
                    }
                } else {
                    Err(CompileError::bare("Self used outside of type definition"))
                }
            }
        }
    }

    fn resolve_type_name(&self, name: &str) -> Result<Type, CompileError> {
        match name {
            "i8" => Ok(Type::I8),
            "i16" => Ok(Type::I16),
            "i32" => Ok(Type::I32),
            "i64" => Ok(Type::I64),
            "u8" => Ok(Type::U8),
            "u16" => Ok(Type::U16),
            "u32" => Ok(Type::U32),
            "u64" => Ok(Type::U64),
            "f64" => Ok(Type::F64),
            "bool" => Ok(Type::Bool),
            "string" => Ok(Type::String),
            "nil" => Ok(Type::Nil),
            "Stdout" => Ok(Type::Capability("Stdout".to_string())),
            "Stdin" => Ok(Type::Capability("Stdin".to_string())),
            "Fs" => Ok(Type::Capability("Fs".to_string())),
            "Env" => Ok(Type::Capability("Env".to_string())),
            "Highlight" => Ok(Type::Capability("Highlight".to_string())),
            _ => {
                if let Some(alias) = self.type_info.aliases.get(name) {
                    return Ok(alias.clone());
                }
                if self.type_info.structs.contains_key(name) {
                    return Ok(Type::Struct(name.to_string()));
                }
                if self.type_info.enums.contains_key(name) {
                    return Ok(Type::Enum(name.to_string()));
                }
                let type_names = self
                    .type_info
                    .structs
                    .keys()
                    .chain(self.type_info.enums.keys())
                    .chain(self.type_info.aliases.keys())
                    .map(|k| k.as_str());
                let msg = match suggest_similar(name, type_names) {
                    Some(s) => format!("unknown type '{}'; did you mean '{}'?", name, s),
                    None => format!("unknown type '{}'", name),
                };
                Err(CompileError::bare(msg))
            }
        }
    }

    fn is_assignable(&self, from: &Type, to: &Type) -> bool {
        if from == to {
            return true;
        }

        // Resolve aliases
        let from = self.resolve_alias(from);
        let to = self.resolve_alias(to);

        if from == to {
            return true;
        }

        match (&from, &to) {
            // nil is compatible with T?
            (Type::Nil, Type::Optional(_)) => true,
            // T is compatible with T?
            (_, Type::Optional(inner)) => self.is_assignable(&from, inner),
            // Error(E) is compatible with T!E
            (Type::Error(e1), Type::Result(_, e2)) => self.is_assignable(e1, e2),
            // T is compatible with T!E (success)
            (_, Type::Result(ok, _)) => self.is_assignable(&from, ok),
            // T is compatible with union containing T
            (_, Type::Union(members)) => members.iter().any(|m| self.is_assignable(&from, m)),
            // Union member is extractable
            (Type::Union(members), _) => {
                // A union is assignable to T only if ALL members are assignable to T
                members.iter().all(|m| self.is_assignable(m, &to))
            }
            // Arrays are invariant: [T] is only assignable to [T], not [T | U].
            // This prevents the covariance hole where writing through a wider type
            // corrupts data visible through the original narrow reference.
            (Type::Array(inner_from), Type::Array(inner_to)) => {
                **inner_from == Type::Nil || **inner_from == **inner_to
            }
            // Maps are invariant for the same reason as arrays: shared references.
            (Type::Map(kf, vf), Type::Map(kt, vt)) => {
                (**kf == Type::Nil && **vf == Type::Nil) || (**kf == **kt && **vf == **vt)
            }
            // Subtype compatibility
            (Type::Struct(sub), Type::Struct(parent))
            | (Type::Capability(sub), Type::Capability(parent)) => self.is_subtype(sub, parent),
            _ => false,
        }
    }

    fn resolve_alias(&self, ty: &Type) -> Type {
        match ty {
            Type::Struct(name) | Type::Enum(name) => {
                if let Some(resolved) = self.type_info.aliases.get(name) {
                    resolved.clone()
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }

    fn is_subtype(&self, sub: &str, parent: &str) -> bool {
        if sub == parent {
            return true;
        }
        if let Some(info) = self.type_info.structs.get(sub) {
            if let Some(ref p) = info.parent {
                return p == parent || self.is_subtype(p, parent);
            }
        }
        false
    }

    fn check_program(&mut self, program: &Program) -> Result<TypedProgram, CompileErrors> {
        // First pass: register all type definitions and function signatures
        self.register_types(program)?;
        self.register_module_types(program)?;
        // Only register std/math if the program imports it
        let has_math_import = program.items.iter().any(|item| {
            if let Item::Import(imp) = item {
                imp.path == "std/math"
            } else {
                false
            }
        });
        if has_math_import {
            self.register_std_math();
        }
        self.register_functions(program)?;

        // Register global functions in scope
        let func_entries: Vec<(String, Type)> = self
            .type_info
            .functions
            .iter()
            .map(|(name, info)| {
                let func_type = Type::Function(
                    info.params.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(info.return_type.clone()),
                );
                (name.clone(), func_type)
            })
            .collect();
        for (name, func_type) in func_entries {
            self.define_var(&name, func_type, false)?;
        }

        // Second pass: type check function bodies
        let mut typed_items = Vec::new();
        let mut errors = Vec::new();
        for item in &program.items {
            match self.check_item(item) {
                Ok(typed) => typed_items.push(typed),
                Err(e) => errors.push(e),
            }
        }

        // Third pass: type-check and compile method bodies as functions
        for item in &program.items {
            if let Item::TypeDef(td) = item {
                let methods = match &td.kind {
                    TypeDefKind::Struct(sdef) => &sdef.methods,
                    TypeDefKind::Enum(edef) => &edef.methods,
                    _ => continue,
                };
                for method in methods {
                    // Look up the method's registered type info
                    let method_info = match &td.kind {
                        TypeDefKind::Struct(_) => self
                            .type_info
                            .structs
                            .get(&td.name)
                            .and_then(|s| s.methods.get(&method.name))
                            .cloned(),
                        TypeDefKind::Enum(_) => self
                            .type_info
                            .enums
                            .get(&td.name)
                            .and_then(|e| e.methods.get(&method.name))
                            .cloned(),
                        _ => None,
                    };
                    if let Some(info) = method_info {
                        self.current_type_name = Some(td.name.clone());
                        match self.check_method_def(method, &info) {
                            Ok(typed_fn) => {
                                self.current_type_name = None;
                                typed_items.push(TypedItem::Function(typed_fn));
                            }
                            Err(e) => {
                                self.current_type_name = None;
                                errors.push(e);
                            }
                        }
                    }
                }
            }
        }

        // Fourth pass: type-check and add module function bodies
        for (module_name, items) in &program.modules {
            for item in items {
                if let Item::Function(fdef) = item {
                    let qualified_name = format!("{}.{}", module_name, fdef.name);
                    let func_info = self
                        .type_info
                        .modules
                        .get(module_name)
                        .and_then(|m| m.functions.get(&fdef.name))
                        .cloned();
                    if let Some(info) = func_info {
                        match self.check_fn_def_with_info(fdef, &info) {
                            Ok(mut typed_fn) => {
                                typed_fn.name = qualified_name;
                                typed_items.push(TypedItem::Function(typed_fn));
                            }
                            Err(e) => errors.push(e),
                        }
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(CompileErrors::new(errors));
        }

        Ok(TypedProgram {
            items: typed_items,
            type_info: self.type_info.clone(),
        })
    }

    fn register_types(&mut self, program: &Program) -> Result<(), CompileError> {
        // Phase 1a: Register all type NAMES (empty shells)
        for item in &program.items {
            if let Item::TypeDef(td) = item {
                self.register_type_name(td)?;
            }
        }
        // Phase 1b: Resolve fields and variants (all type names now visible)
        for item in &program.items {
            if let Item::TypeDef(td) = item {
                self.register_type_def_skeleton(td)?;
            }
        }
        // Phase 2: Register methods (can now reference all types and fields)
        for item in &program.items {
            if let Item::TypeDef(td) = item {
                self.register_type_def_methods(td)?;
            }
        }
        Ok(())
    }

    fn register_module_types(&mut self, program: &Program) -> Result<(), CompileError> {
        // Phase 1: Register all module type NAMES as empty shells so they can
        // reference each other during field resolution.
        for (module_name, items) in &program.modules {
            for item in items {
                if let Item::TypeDef(td) = item {
                    let qualified = format!("{}.{}", module_name, td.name);
                    match &td.kind {
                        TypeDefKind::Struct(_) => {
                            self.type_info.structs.insert(
                                qualified,
                                StructInfo {
                                    name: td.name.clone(),
                                    fields: Vec::new(),
                                    methods: HashMap::new(),
                                    parent: td.parent.clone(),
                                },
                            );
                        }
                        TypeDefKind::Enum(_) => {
                            self.type_info.enums.insert(
                                qualified,
                                EnumInfo {
                                    name: td.name.clone(),
                                    variants: Vec::new(),
                                    methods: HashMap::new(),
                                },
                            );
                        }
                        TypeDefKind::Alias(_) => {}
                    }
                }
            }
        }

        // Phase 2: Resolve fields, variants, methods, and functions.
        for (module_name, items) in &program.modules {
            let mut mod_info = ModuleInfo {
                structs: HashMap::new(),
                enums: HashMap::new(),
                aliases: HashMap::new(),
                functions: HashMap::new(),
            };

            for item in items {
                match item {
                    Item::TypeDef(td) => match &td.kind {
                        TypeDefKind::Struct(sdef) => {
                            let fields: Vec<(String, Type)> = sdef
                                .fields
                                .iter()
                                .map(|f| {
                                    Ok((
                                        f.name.clone(),
                                        self.resolve_type_expr_in_module(&f.ty, module_name)?,
                                    ))
                                })
                                .collect::<Result<_, CompileError>>()?;
                            let mut methods = HashMap::new();
                            for m in &sdef.methods {
                                let params: Vec<(String, Type)> = m
                                    .params
                                    .iter()
                                    .map(|p| {
                                        Ok((
                                            p.name.clone(),
                                            self.resolve_type_expr_in_module(&p.ty, module_name)?,
                                        ))
                                    })
                                    .collect::<Result<_, CompileError>>()?;
                                let ret = m
                                    .return_type
                                    .as_ref()
                                    .map(|t| self.resolve_type_expr_in_module(t, module_name))
                                    .transpose()?
                                    .unwrap_or(Type::Nil);
                                methods.insert(
                                    m.name.clone(),
                                    FuncInfo {
                                        name: m.name.clone(),
                                        params,
                                        return_type: ret,
                                        is_method: true,
                                    },
                                );
                            }
                            mod_info.structs.insert(
                                td.name.clone(),
                                StructInfo {
                                    name: td.name.clone(),
                                    fields,
                                    methods,
                                    parent: td.parent.clone(),
                                },
                            );
                        }
                        TypeDefKind::Enum(edef) => {
                            let variants: Vec<VariantInfo> = edef
                                .variants
                                .iter()
                                .map(|v| {
                                    let fields: Result<Vec<Type>, CompileError> = v
                                        .fields
                                        .iter()
                                        .map(|t| self.resolve_type_expr_in_module(t, module_name))
                                        .collect();
                                    Ok(VariantInfo {
                                        name: v.name.clone(),
                                        fields: fields?,
                                    })
                                })
                                .collect::<Result<_, CompileError>>()?;
                            mod_info.enums.insert(
                                td.name.clone(),
                                EnumInfo {
                                    name: td.name.clone(),
                                    variants,
                                    methods: HashMap::new(),
                                },
                            );
                        }
                        TypeDefKind::Alias(ty) => {
                            let resolved = self.resolve_type_expr_in_module(ty, module_name)?;
                            mod_info.aliases.insert(td.name.clone(), resolved);
                        }
                    },
                    Item::Function(fdef) => {
                        let params: Vec<(String, Type)> = fdef
                            .params
                            .iter()
                            .map(|p| {
                                Ok((
                                    p.name.clone(),
                                    self.resolve_type_expr_in_module(&p.ty, module_name)?,
                                ))
                            })
                            .collect::<Result<_, CompileError>>()?;
                        let ret = fdef
                            .return_type
                            .as_ref()
                            .map(|t| self.resolve_type_expr_in_module(t, module_name))
                            .transpose()?
                            .unwrap_or(Type::Nil);
                        mod_info.functions.insert(
                            fdef.name.clone(),
                            FuncInfo {
                                name: fdef.name.clone(),
                                params,
                                return_type: ret,
                                is_method: false,
                            },
                        );
                    }
                    _ => {}
                }
            }

            // Sync module struct/enum info back to global type_info for codegen
            for (name, info) in &mod_info.structs {
                let qualified = format!("{}.{}", module_name, name);
                self.type_info.structs.insert(qualified, info.clone());
            }
            for (name, info) in &mod_info.enums {
                let qualified = format!("{}.{}", module_name, name);
                self.type_info.enums.insert(qualified, info.clone());
            }

            self.type_info.modules.insert(module_name.clone(), mod_info);
        }
        Ok(())
    }

    /// Register the built-in math standard library module.
    fn register_std_math(&mut self) {
        // Only register if not already provided by an explicit module
        if self.type_info.modules.contains_key("math") {
            return;
        }
        let mut functions = HashMap::new();
        let float_ty = Type::F64;

        // Helper: one-arg float -> float
        let one_arg = |name: &str| -> (String, FuncInfo) {
            (
                name.to_string(),
                FuncInfo {
                    name: name.to_string(),
                    params: vec![("x".to_string(), Type::F64)],
                    return_type: float_ty.clone(),
                    is_method: false,
                },
            )
        };

        // Helper: zero-arg -> float (constants)
        let zero_arg = |name: &str| -> (String, FuncInfo) {
            (
                name.to_string(),
                FuncInfo {
                    name: name.to_string(),
                    params: vec![],
                    return_type: float_ty.clone(),
                    is_method: false,
                },
            )
        };

        // Helper: two-arg (float, float) -> float
        let two_arg = |name: &str| -> (String, FuncInfo) {
            (
                name.to_string(),
                FuncInfo {
                    name: name.to_string(),
                    params: vec![("a".to_string(), Type::F64), ("b".to_string(), Type::F64)],
                    return_type: float_ty.clone(),
                    is_method: false,
                },
            )
        };

        // Existing functions
        functions.insert(one_arg("sqrt").0, one_arg("sqrt").1);
        functions.insert(one_arg("abs").0, one_arg("abs").1);
        functions.insert(one_arg("floor").0, one_arg("floor").1);
        functions.insert(one_arg("ceil").0, one_arg("ceil").1);
        functions.insert(one_arg("round").0, one_arg("round").1);
        functions.insert(two_arg("min").0, two_arg("min").1);
        functions.insert(two_arg("max").0, two_arg("max").1);

        // Trigonometry
        for name in ["sin", "cos", "tan", "asin", "acos", "atan"] {
            let (k, v) = one_arg(name);
            functions.insert(k, v);
        }
        {
            let (k, v) = two_arg("atan2");
            functions.insert(k, v);
        }

        // Logarithms & exponentials
        for name in ["log", "log2", "log10", "exp"] {
            let (k, v) = one_arg(name);
            functions.insert(k, v);
        }
        {
            let (k, v) = two_arg("pow");
            functions.insert(k, v);
        }

        // Constants (zero-arg functions)
        for name in ["pi", "e", "tau", "inf"] {
            let (k, v) = zero_arg(name);
            functions.insert(k, v);
        }

        self.type_info.modules.insert(
            "math".to_string(),
            ModuleInfo {
                structs: HashMap::new(),
                enums: HashMap::new(),
                aliases: HashMap::new(),
                functions,
            },
        );
    }

    /// Phase 1a: Register just the type name as an empty shell.
    fn register_type_name(&mut self, td: &TypeDef) -> Result<(), CompileError> {
        const RESERVED_TYPE_NAMES: &[&str] = &["Stdout", "Stdin", "Fs", "Env", "Highlight"];

        if RESERVED_TYPE_NAMES.contains(&td.name.as_str()) {
            return Err(CompileError::new(
                format!(
                    "type name '{}' is reserved for a built-in capability",
                    td.name
                ),
                td.span,
            ));
        }

        match &td.kind {
            TypeDefKind::Struct(_) => {
                self.type_info
                    .structs
                    .entry(td.name.clone())
                    .or_insert_with(|| StructInfo {
                        name: td.name.clone(),
                        fields: Vec::new(),
                        methods: HashMap::new(),
                        parent: td.parent.clone(),
                    });
            }
            TypeDefKind::Enum(_) => {
                self.type_info
                    .enums
                    .entry(td.name.clone())
                    .or_insert_with(|| EnumInfo {
                        name: td.name.clone(),
                        variants: Vec::new(),
                        methods: HashMap::new(),
                    });
            }
            TypeDefKind::Alias(ty) => {
                // For aliases, we can try to resolve but it may fail if the alias
                // references another not-yet-registered type. That's OK — skeleton phase
                // will handle it.
                if let Ok(resolved) = self.resolve_type_expr(ty) {
                    self.type_info.aliases.insert(td.name.clone(), resolved);
                }
            }
        }
        Ok(())
    }

    /// Phase 1b: Register type fields and variants (all names now visible).
    fn register_type_def_skeleton(&mut self, td: &TypeDef) -> Result<(), CompileError> {
        match &td.kind {
            TypeDefKind::Struct(sdef) => {
                self.current_type_name = Some(td.name.clone());
                // Name already registered in phase 1a. Now resolve field types.
                let fields: Vec<(String, Type)> = sdef
                    .fields
                    .iter()
                    .map(|f| Ok((f.name.clone(), self.resolve_type_expr(&f.ty)?)))
                    .collect::<Result<_, CompileError>>()?;
                // SAFETY: struct name was registered in phase 1a (register_type_names)
                self.type_info.structs.get_mut(&td.name).unwrap().fields = fields;
                self.current_type_name = None;
            }
            TypeDefKind::Enum(edef) => {
                self.current_type_name = Some(td.name.clone());
                // Name already registered in phase 1a. Now resolve variant field types.
                let variants: Vec<VariantInfo> = edef
                    .variants
                    .iter()
                    .map(|v| {
                        let fields: Result<Vec<Type>, CompileError> =
                            v.fields.iter().map(|t| self.resolve_type_expr(t)).collect();
                        Ok(VariantInfo {
                            name: v.name.clone(),
                            fields: fields?,
                        })
                    })
                    .collect::<Result<_, CompileError>>()?;
                // SAFETY: enum name was registered in phase 1a (register_type_names)
                self.type_info.enums.get_mut(&td.name).unwrap().variants = variants;
                self.current_type_name = None;
            }
            TypeDefKind::Alias(ty) => {
                let resolved = self.resolve_type_expr(ty)?;
                self.type_info.aliases.insert(td.name.clone(), resolved);
            }
        }
        Ok(())
    }

    /// Phase 2: Register methods on already-registered types.
    fn register_type_def_methods(&mut self, td: &TypeDef) -> Result<(), CompileError> {
        self.current_type_name = Some(td.name.clone());
        match &td.kind {
            TypeDefKind::Struct(sdef) => {
                let mut methods = HashMap::new();
                for m in &sdef.methods {
                    let params: Vec<(String, Type)> = m
                        .params
                        .iter()
                        .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                        .collect::<Result<_, CompileError>>()?;
                    let ret = m
                        .return_type
                        .as_ref()
                        .map(|t| self.resolve_type_expr(t))
                        .transpose()?
                        .unwrap_or(Type::Nil);
                    let is_method = params.first().map(|(n, _)| n == "self").unwrap_or(false);
                    methods.insert(
                        m.name.clone(),
                        FuncInfo {
                            name: m.name.clone(),
                            params,
                            return_type: ret,
                            is_method,
                        },
                    );
                }
                if let Some(info) = self.type_info.structs.get_mut(&td.name) {
                    info.methods = methods;
                }
            }
            TypeDefKind::Enum(edef) => {
                let mut methods = HashMap::new();
                for m in &edef.methods {
                    let params: Vec<(String, Type)> = m
                        .params
                        .iter()
                        .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                        .collect::<Result<_, CompileError>>()?;
                    let ret = m
                        .return_type
                        .as_ref()
                        .map(|t| self.resolve_type_expr(t))
                        .transpose()?
                        .unwrap_or(Type::Nil);
                    let is_method = params.first().map(|(n, _)| n == "self").unwrap_or(false);
                    methods.insert(
                        m.name.clone(),
                        FuncInfo {
                            name: m.name.clone(),
                            params,
                            return_type: ret,
                            is_method,
                        },
                    );
                }
                if let Some(info) = self.type_info.enums.get_mut(&td.name) {
                    info.methods = methods;
                }
            }
            TypeDefKind::Alias(_) => {} // No methods on aliases
        }
        self.current_type_name = None;
        Ok(())
    }

    fn register_functions(&mut self, program: &Program) -> Result<(), CompileError> {
        for item in &program.items {
            if let Item::Function(fdef) = item {
                let params: Vec<(String, Type)> = fdef
                    .params
                    .iter()
                    .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                    .collect::<Result<_, CompileError>>()?;
                let ret = fdef
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type_expr(t))
                    .transpose()
                    .map_err(|e| e.with_span(fdef.span))?
                    .unwrap_or(Type::Nil);
                self.type_info.functions.insert(
                    fdef.name.clone(),
                    FuncInfo {
                        name: fdef.name.clone(),
                        params,
                        return_type: ret,
                        is_method: false,
                    },
                );
            }
        }
        Ok(())
    }

    fn check_item(&mut self, item: &Item) -> Result<TypedItem, CompileError> {
        match item {
            Item::Function(fdef) => {
                let typed = self.check_fn_def(fdef)?;
                Ok(TypedItem::Function(typed))
            }
            Item::TypeDef(td) => {
                // Type-check method bodies are handled separately
                Ok(TypedItem::TypeDef(td.clone()))
            }
            Item::Let(decl) => {
                let typed = self.check_let_decl(decl)?;
                Ok(TypedItem::Let(typed))
            }
            Item::Import(imp) => Ok(TypedItem::Import(imp.clone())),
        }
    }

    fn check_fn_def(&mut self, fdef: &FnDef) -> Result<TypedFnDef, CompileError> {
        let info = self
            .type_info
            .functions
            .get(&fdef.name)
            .cloned()
            .ok_or_else(|| {
                CompileError::new(format!("unknown function '{}'", fdef.name), fdef.span)
            })?;
        self.check_fn_def_with_info(fdef, &info)
    }

    fn check_fn_def_with_info(
        &mut self,
        fdef: &FnDef,
        info: &FuncInfo,
    ) -> Result<TypedFnDef, CompileError> {
        self.push_scope();
        let old_return = self.current_fn_return.take();
        self.current_fn_return = Some(info.return_type.clone());

        // Bind parameters
        for (name, ty) in &info.params {
            self.define_var(name, ty.clone(), false)?;
        }

        // Scan for mutable variables captured by closures (for narrowing safety)
        let old_captured = std::mem::take(&mut self.captured_mut_vars);
        self.captured_mut_vars = find_captured_mut_vars(&fdef.body, &self.scopes);

        let body = self.check_block(&fdef.body)?;

        // Check that non-void functions return on all paths
        if info.return_type != Type::Nil
            && info.return_type != Type::Void
            && !self.block_always_returns(&body)
        {
            return Err(CompileError::new(
                format!(
                    "function '{}' declares return type {} but not all code paths return a value",
                    fdef.name,
                    info.return_type.display_name()
                ),
                fdef.span,
            ));
        }

        self.current_fn_return = old_return;
        self.captured_mut_vars = old_captured;
        self.pop_scope();

        Ok(TypedFnDef {
            name: fdef.name.clone(),
            params: info.params.clone(),
            return_type: info.return_type.clone(),
            body,
        })
    }

    fn check_method_def(
        &mut self,
        method: &FnDef,
        info: &FuncInfo,
    ) -> Result<TypedFnDef, CompileError> {
        self.push_scope();
        let old_return = self.current_fn_return.take();
        self.current_fn_return = Some(info.return_type.clone());

        // Bind parameters
        for (name, ty) in &info.params {
            self.define_var(name, ty.clone(), false)?;
        }

        let body = self.check_block(&method.body)?;

        // Check that non-void methods return on all paths
        if info.return_type != Type::Nil
            && info.return_type != Type::Void
            && !self.block_always_returns(&body)
        {
            return Err(CompileError::new(
                format!(
                    "method '{}' declares return type {} but not all code paths return a value",
                    method.name,
                    info.return_type.display_name()
                ),
                method.span,
            ));
        }

        self.current_fn_return = old_return;
        self.pop_scope();

        Ok(TypedFnDef {
            name: method.name.clone(),
            params: info.params.clone(),
            return_type: info.return_type.clone(),
            body,
        })
    }

    fn check_let_decl(&mut self, decl: &LetDecl) -> Result<TypedLetDecl, CompileError> {
        let mut value = self.check_expr(&decl.value)?;

        let ty = if let Some(ref type_expr) = decl.ty {
            let declared = self
                .resolve_type_expr(type_expr)
                .map_err(|e| e.with_span(decl.span))?;
            // Special case: integer literal (or negated literal) assigned to narrow integer type
            let int_literal_value = Self::extract_int_literal(&value);
            let is_int_literal_narrowing =
                int_literal_value.is_some() && value.ty.is_integer() && declared.is_integer();
            if !is_int_literal_narrowing && !self.is_assignable(&value.ty, &declared) {
                return Err(CompileError::new(
                    format!(
                        "type mismatch in let '{}': expected {}, got {}",
                        decl.name,
                        declared.display_name(),
                        value.ty.display_name()
                    ),
                    decl.span,
                ));
            }
            // Compile-time range check for integer literals assigned to narrow types
            if let Some(n) = int_literal_value {
                Self::check_int_literal_range(n, &declared, &decl.name, decl.span)?;
                // Update the expression's type to match the declared type so codegen
                // emits the correct opcode (LoadInt vs LoadUInt)
                value.ty = declared.clone();
            }
            declared
        } else {
            value.ty.clone()
        };

        self.define_var(&decl.name, ty.clone(), decl.mutable)?;

        Ok(TypedLetDecl {
            name: decl.name.clone(),
            mutable: decl.mutable,
            ty,
            value,
        })
    }

    fn check_block(&mut self, block: &Block) -> Result<TypedBlock, CompileError> {
        self.push_scope();
        let mut stmts = Vec::new();
        let mut block_ty = Type::Nil;

        for (i, stmt) in block.stmts.iter().enumerate() {
            let typed = self.check_stmt(stmt)?;
            // The block's type is the type of the last expression statement
            if i == block.stmts.len() - 1 {
                block_ty = match &typed {
                    TypedStmt::Expr(e) => e.ty.clone(),
                    _ => Type::Nil,
                };
            }
            stmts.push(typed);
        }

        self.pop_scope();
        Ok(TypedBlock {
            stmts,
            ty: block_ty,
        })
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<TypedStmt, CompileError> {
        match &stmt.kind {
            StmtKind::Let(decl) => {
                let typed = self.check_let_decl(decl)?;
                Ok(TypedStmt::Let(typed))
            }
            StmtKind::LetTuple(names, value) => {
                let typed_value = self.check_expr(value)?;
                match &typed_value.ty {
                    Type::Tuple(types) => {
                        if names.len() != types.len() {
                            return Err(CompileError::new(
                                format!(
                                    "tuple destructure: expected {} elements, got {}",
                                    names.len(),
                                    types.len()
                                ),
                                stmt.span,
                            ));
                        }
                        let mut bindings = Vec::new();
                        for (i, name) in names.iter().enumerate() {
                            let ty = types[i].clone();
                            self.define_var(name, ty.clone(), false)?;
                            bindings.push((name.clone(), ty));
                        }
                        Ok(TypedStmt::LetTuple(bindings, typed_value))
                    }
                    _ => Err(CompileError::new(
                        format!(
                            "cannot destructure non-tuple type {}",
                            typed_value.ty.display_name()
                        ),
                        stmt.span,
                    )),
                }
            }
            StmtKind::Assign(target, value) => {
                let typed_value = self.check_expr(value)?;
                let typed_target = match target {
                    AssignTarget::Variable(name) => {
                        let (var_ty, mutable) = self.lookup_var(name).ok_or_else(|| {
                            let all_vars: Vec<&str> = self
                                .scopes
                                .iter()
                                .flat_map(|s| s.keys().map(|k| k.as_str()))
                                .collect();
                            let msg = match suggest_similar(name, all_vars.into_iter()) {
                                Some(s) => {
                                    format!("undefined variable '{}'; did you mean '{}'?", name, s)
                                }
                                None => format!("undefined variable '{}'", name),
                            };
                            CompileError::new(msg, stmt.span)
                        })?;
                        if !mutable {
                            return Err(CompileError::new(
                                format!("cannot assign to immutable variable '{}'", name),
                                stmt.span,
                            ));
                        }
                        if !self.is_assignable(&typed_value.ty, &var_ty) {
                            return Err(CompileError::new(
                                format!(
                                    "type mismatch in assignment to '{}': expected {}, got {}",
                                    name,
                                    var_ty.display_name(),
                                    typed_value.ty.display_name()
                                ),
                                stmt.span,
                            ));
                        }
                        TypedAssignTarget::Variable(name.clone(), var_ty)
                    }
                    AssignTarget::Field(obj, field) => {
                        self.check_mutable_target(obj, "assign to field")?;
                        let typed_obj = self.check_expr(obj)?;
                        let field_ty = self.get_field_type(&typed_obj.ty, field, stmt.span)?;
                        if !self.is_assignable(&typed_value.ty, &field_ty) {
                            return Err(CompileError::new(
                                format!(
                                    "type mismatch in field assignment: expected {}, got {}",
                                    field_ty.display_name(),
                                    typed_value.ty.display_name()
                                ),
                                stmt.span,
                            ));
                        }
                        TypedAssignTarget::Field(typed_obj, field.clone(), field_ty)
                    }
                    AssignTarget::Index(obj, idx) => {
                        self.check_mutable_target(obj, "assign to index")?;
                        let typed_obj = self.check_expr(obj)?;
                        let typed_idx = self.check_expr(idx)?;
                        let elem_ty =
                            self.get_index_type(&typed_obj.ty, &typed_idx.ty, stmt.span)?;
                        if !self.is_assignable(&typed_value.ty, &elem_ty) {
                            return Err(CompileError::new(
                                format!(
                                    "type mismatch in index assignment: expected {}, got {}",
                                    elem_ty.display_name(),
                                    typed_value.ty.display_name()
                                ),
                                stmt.span,
                            ));
                        }
                        TypedAssignTarget::Index(typed_obj, typed_idx, elem_ty)
                    }
                };
                Ok(TypedStmt::Assign(
                    Box::new(typed_target),
                    Box::new(typed_value),
                ))
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    let typed = self.check_expr(expr)?;
                    if let Some(ref ret_ty) = self.current_fn_return {
                        if !self.is_assignable(&typed.ty, ret_ty) {
                            return Err(CompileError::new(
                                format!(
                                    "return type mismatch: expected {}, got {}",
                                    ret_ty.display_name(),
                                    typed.ty.display_name()
                                ),
                                stmt.span,
                            ));
                        }
                    }
                    Ok(TypedStmt::Return(Some(typed)))
                } else {
                    Ok(TypedStmt::Return(None))
                }
            }
            StmtKind::ReturnError(expr) => {
                let typed = self.check_expr(expr)?;
                // Check that enclosing function returns a result type
                if let Some(Type::Result(_, err_ty)) = &self.current_fn_return {
                    if !self.is_assignable(&typed.ty, err_ty) {
                        return Err(CompileError::new(
                            format!(
                                "error type mismatch: expected {}, got {}",
                                err_ty.display_name(),
                                typed.ty.display_name()
                            ),
                            stmt.span,
                        ));
                    }
                }
                Ok(TypedStmt::ReturnError(typed))
            }
            StmtKind::Break => {
                if !self.in_loop {
                    return Err(CompileError::new("break outside of loop", stmt.span));
                }
                Ok(TypedStmt::Break)
            }
            StmtKind::Continue => {
                if !self.in_loop {
                    return Err(CompileError::new("continue outside of loop", stmt.span));
                }
                Ok(TypedStmt::Continue)
            }
            StmtKind::Expr(expr) => {
                let typed = self.check_expr(expr)?;
                // Spec: "Errors cannot be silently ignored."
                // Result types used as expression statements must be consumed.
                if let Type::Result(_, _) = &typed.ty {
                    return Err(CompileError::new(
                        "result type must be used: assign to a variable, match, or use '?'",
                        stmt.span,
                    ));
                }
                Ok(TypedStmt::Expr(typed))
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<TypedExpr, CompileError> {
        match &expr.kind {
            ExprKind::IntLit(n) => Ok(TypedExpr {
                kind: TypedExprKind::IntLit(*n),
                ty: Type::I64,
                span: expr.span,
            }),
            ExprKind::FloatLit(f) => Ok(TypedExpr {
                kind: TypedExprKind::FloatLit(*f),
                ty: Type::F64,
                span: expr.span,
            }),
            ExprKind::BoolLit(b) => Ok(TypedExpr {
                kind: TypedExprKind::BoolLit(*b),
                ty: Type::Bool,
                span: expr.span,
            }),
            ExprKind::StringLit(s) => Ok(TypedExpr {
                kind: TypedExprKind::StringLit(s.clone()),
                ty: Type::String,
                span: expr.span,
            }),
            ExprKind::InterpolatedString(parts) => {
                let mut typed_parts = Vec::new();
                for part in parts {
                    match part {
                        StringPart::Literal(s) => {
                            typed_parts.push(TypedStringPart::Literal(s.clone()));
                        }
                        StringPart::Expr(expr) => {
                            let typed = self.check_expr(expr)?;
                            typed_parts.push(TypedStringPart::Expr(typed));
                        }
                    }
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::InterpolatedString(typed_parts),
                    ty: Type::String,
                    span: expr.span,
                })
            }
            ExprKind::Nil => Ok(TypedExpr {
                kind: TypedExprKind::Nil,
                ty: Type::Nil,
                span: expr.span,
            }),
            ExprKind::Ident(name) => {
                if name == "panic" {
                    // panic is a global builtin
                    Ok(TypedExpr {
                        kind: TypedExprKind::Ident("panic".to_string()),
                        ty: Type::Function(vec![Type::String], Box::new(Type::Nil)),
                        span: expr.span,
                    })
                } else if let Some((ty, _)) = self.lookup_var(name) {
                    Ok(TypedExpr {
                        kind: TypedExprKind::Ident(name.clone()),
                        ty,
                        span: expr.span,
                    })
                } else if let Some(func_info) = self.type_info.functions.get(name) {
                    let func_type = Type::Function(
                        func_info.params.iter().map(|(_, t)| t.clone()).collect(),
                        Box::new(func_info.return_type.clone()),
                    );
                    Ok(TypedExpr {
                        kind: TypedExprKind::Ident(name.clone()),
                        ty: func_type,
                        span: expr.span,
                    })
                } else {
                    // Could be module name
                    if self.type_info.modules.contains_key(name) {
                        Ok(TypedExpr {
                            kind: TypedExprKind::Ident(name.clone()),
                            ty: Type::Nil, // module is not a value
                            span: expr.span,
                        })
                    } else {
                        let all_vars: Vec<&str> = self
                            .scopes
                            .iter()
                            .flat_map(|s| s.keys().map(|k| k.as_str()))
                            .collect();
                        let msg = match suggest_similar(name, all_vars.into_iter()) {
                            Some(s) => {
                                format!("undefined variable '{}'; did you mean '{}'?", name, s)
                            }
                            None => format!("undefined variable '{}'", name),
                        };
                        Err(CompileError::new(msg, expr.span))
                    }
                }
            }
            ExprKind::TypeIdent(name) => {
                // Type as value (for static access like Type.method)
                Ok(TypedExpr {
                    kind: TypedExprKind::Ident(name.clone()),
                    ty: Type::Nil,
                    span: expr.span,
                })
            }
            ExprKind::BinOp(left, op, right) => {
                let typed_left = self.check_expr(left)?;
                let typed_right = self.check_expr(right)?;
                let result_ty = self.check_binop(&typed_left.ty, op, &typed_right.ty, expr.span)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::BinOp(
                        Box::new(typed_left),
                        op.clone(),
                        Box::new(typed_right),
                    ),
                    ty: result_ty,
                    span: expr.span,
                })
            }
            ExprKind::UnaryOp(op, expr) => {
                let typed = self.check_expr(expr)?;
                let result_ty = match op {
                    UnaryOp::Neg => {
                        if !typed.ty.is_numeric() {
                            return Err(CompileError::new(
                                format!("cannot negate {}", typed.ty.display_name()),
                                expr.span,
                            ));
                        }
                        typed.ty.clone()
                    }
                    UnaryOp::Not => {
                        if typed.ty != Type::Bool {
                            return Err(CompileError::new(
                                format!(
                                    "logical NOT requires bool, got {}",
                                    typed.ty.display_name()
                                ),
                                expr.span,
                            ));
                        }
                        Type::Bool
                    }
                    UnaryOp::BitNot => {
                        if !typed.ty.is_integer() {
                            return Err(CompileError::new(
                                format!(
                                    "bitwise NOT requires integer, got {}",
                                    typed.ty.display_name()
                                ),
                                expr.span,
                            ));
                        }
                        typed.ty.clone()
                    }
                };
                Ok(TypedExpr {
                    kind: TypedExprKind::UnaryOp(op.clone(), Box::new(typed)),
                    ty: result_ty,
                    span: expr.span,
                })
            }
            ExprKind::FieldAccess(obj, field) => {
                let typed_obj = self.check_expr(obj)?;

                // Check for .length property
                if field == "length" {
                    match &typed_obj.ty {
                        Type::String | Type::Array(_) | Type::Map(_, _) | Type::Tuple(_) => {
                            return Ok(TypedExpr {
                                kind: TypedExprKind::FieldAccess(
                                    Box::new(typed_obj),
                                    "length".to_string(),
                                ),
                                ty: Type::I64,
                                span: expr.span,
                            });
                        }
                        _ => {}
                    }
                }

                // Check for tuple index access
                if let Type::Tuple(types) = &typed_obj.ty {
                    if let Ok(idx) = field.parse::<usize>() {
                        if idx < types.len() {
                            let elem_ty = types[idx].clone();
                            return Ok(TypedExpr {
                                kind: TypedExprKind::FieldAccess(
                                    Box::new(typed_obj),
                                    field.clone(),
                                ),
                                ty: elem_ty,
                                span: expr.span,
                            });
                        }
                        return Err(CompileError::new(
                            format!("tuple index {} out of bounds (length {})", idx, types.len()),
                            expr.span,
                        ));
                    }
                }

                let field_ty = self.get_field_type(&typed_obj.ty, field, expr.span)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::FieldAccess(Box::new(typed_obj), field.clone()),
                    ty: field_ty,
                    span: expr.span,
                })
            }
            ExprKind::TypeAccess(obj, name) => {
                // module.Type access or Type.StaticField
                let typed_obj = self.check_expr(obj)?;
                // The type access itself just passes through
                Ok(TypedExpr {
                    kind: TypedExprKind::FieldAccess(Box::new(typed_obj), name.clone()),
                    ty: Type::Nil,
                    span: expr.span,
                })
            }
            ExprKind::Index(obj, idx) => {
                let typed_obj = self.check_expr(obj)?;
                let typed_idx = self.check_expr(idx)?;
                let elem_ty = self.get_index_type(&typed_obj.ty, &typed_idx.ty, expr.span)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::Index(Box::new(typed_obj), Box::new(typed_idx)),
                    ty: elem_ty,
                    span: expr.span,
                })
            }
            ExprKind::Call(func, args) => self.check_call(func, args).map(|mut e| {
                e.span = expr.span;
                e
            }),
            ExprKind::MethodCall(obj, method, args) => {
                self.check_method_call(obj, method, args).map(|mut e| {
                    e.span = expr.span;
                    e
                })
            }
            ExprKind::StaticMethodCall(type_or_module, method, args) => self
                .check_static_method_call(type_or_module, method, args, expr.span)
                .map(|mut e| {
                    e.span = expr.span;
                    e
                }),
            ExprKind::ArrayLit(elems) => {
                if elems.is_empty() {
                    return Ok(TypedExpr {
                        kind: TypedExprKind::ArrayLit(vec![]),
                        ty: Type::Array(Box::new(Type::Nil)), // inferred from context
                        span: expr.span,
                    });
                }
                let mut typed_elems = Vec::new();
                let first = self.check_expr(&elems[0])?;
                let elem_ty = first.ty.clone();
                typed_elems.push(first);
                for elem in &elems[1..] {
                    let typed = self.check_expr(elem)?;
                    if !self.is_assignable(&typed.ty, &elem_ty) {
                        return Err(CompileError::new(
                            format!(
                                "array element type mismatch: expected {}, got {}",
                                elem_ty.display_name(),
                                typed.ty.display_name()
                            ),
                            expr.span,
                        ));
                    }
                    typed_elems.push(typed);
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::ArrayLit(typed_elems),
                    ty: Type::Array(Box::new(elem_ty)),
                    span: expr.span,
                })
            }
            ExprKind::MapLit(entries) => {
                if entries.is_empty() {
                    return Ok(TypedExpr {
                        kind: TypedExprKind::MapLit(vec![]),
                        ty: Type::Map(Box::new(Type::Nil), Box::new(Type::Nil)),
                        span: expr.span,
                    });
                }
                let mut typed_entries = Vec::new();
                let first_key = self.check_expr(&entries[0].0)?;
                let first_val = self.check_expr(&entries[0].1)?;
                let key_ty = first_key.ty.clone();
                let val_ty = first_val.ty.clone();
                typed_entries.push((first_key, first_val));
                for (k, v) in &entries[1..] {
                    let typed_k = self.check_expr(k)?;
                    let typed_v = self.check_expr(v)?;
                    if !self.is_assignable(&typed_k.ty, &key_ty) {
                        return Err(CompileError::new(
                            format!(
                                "map key type mismatch: expected {}, got {}",
                                key_ty.display_name(),
                                typed_k.ty.display_name()
                            ),
                            expr.span,
                        ));
                    }
                    if !self.is_assignable(&typed_v.ty, &val_ty) {
                        return Err(CompileError::new(
                            format!(
                                "map value type mismatch: expected {}, got {}",
                                val_ty.display_name(),
                                typed_v.ty.display_name()
                            ),
                            expr.span,
                        ));
                    }
                    typed_entries.push((typed_k, typed_v));
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::MapLit(typed_entries),
                    ty: Type::Map(Box::new(key_ty), Box::new(val_ty)),
                    span: expr.span,
                })
            }
            ExprKind::TupleLit(elems) => {
                let mut typed_elems = Vec::new();
                let mut types = Vec::new();
                for elem in elems {
                    let typed = self.check_expr(elem)?;
                    types.push(typed.ty.clone());
                    typed_elems.push(typed);
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::TupleLit(typed_elems),
                    ty: Type::Tuple(types),
                    span: expr.span,
                })
            }
            ExprKind::StructLit(type_name, module, fields, spread) => {
                let full_name = if let Some(m) = module {
                    format!("{}.{}", m, type_name)
                } else {
                    type_name.clone()
                };

                let struct_info = if let Some(m) = module {
                    self.type_info
                        .modules
                        .get(m)
                        .and_then(|mi| mi.structs.get(type_name))
                        .cloned()
                } else {
                    self.type_info.structs.get(type_name).cloned()
                };

                let info = struct_info.ok_or_else(|| {
                    let type_names = self
                        .type_info
                        .structs
                        .keys()
                        .chain(self.type_info.enums.keys())
                        .chain(self.type_info.aliases.keys())
                        .map(|k| k.as_str());
                    let msg = match suggest_similar(&full_name, type_names) {
                        Some(s) => {
                            format!("unknown struct type '{}'; did you mean '{}'?", full_name, s)
                        }
                        None => format!("unknown struct type '{}'", full_name),
                    };
                    CompileError::new(msg, expr.span)
                })?;

                let mut typed_fields = Vec::new();
                let mut provided: std::collections::HashSet<String> =
                    std::collections::HashSet::new();

                for (name, expr) in fields {
                    if !provided.insert(name.clone()) {
                        return Err(CompileError::new(
                            format!("duplicate field '{}' in struct literal", name),
                            expr.span,
                        ));
                    }
                    let field_ty = info
                        .fields
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| {
                            let field_names = info.fields.iter().map(|(n, _)| n.as_str());
                            let msg = match suggest_similar(name, field_names) {
                                Some(s) => format!(
                                    "unknown field '{}' on struct '{}'; did you mean '{}'?",
                                    name, full_name, s
                                ),
                                None => {
                                    format!("unknown field '{}' on struct '{}'", name, full_name)
                                }
                            };
                            CompileError::new(msg, expr.span)
                        })?;
                    let typed = self.check_expr(expr)?;
                    if !self.is_assignable(&typed.ty, &field_ty) {
                        return Err(CompileError::new(
                            format!(
                                "field '{}' type mismatch: expected {}, got {}",
                                name,
                                field_ty.display_name(),
                                typed.ty.display_name()
                            ),
                            expr.span,
                        ));
                    }
                    typed_fields.push((name.clone(), typed));
                }

                // Handle spread (functional update): fill missing fields from spread expr
                if let Some(spread_expr) = spread {
                    let typed_spread = self.check_expr(spread_expr)?;
                    if typed_spread.ty != Type::Struct(full_name.clone()) {
                        return Err(CompileError::new(
                            format!(
                                "spread type mismatch: expected {}, got {}",
                                full_name,
                                typed_spread.ty.display_name()
                            ),
                            expr.span,
                        ));
                    }
                    // Fill in any unprovided fields from the spread
                    for (i, (name, _)) in info.fields.iter().enumerate() {
                        if !provided.contains(name) {
                            provided.insert(name.clone());
                            typed_fields.push((
                                name.clone(),
                                TypedExpr {
                                    kind: TypedExprKind::FieldAccess(
                                        Box::new(typed_spread.clone()),
                                        name.clone(),
                                    ),
                                    ty: info.fields[i].1.clone(),
                                    span: expr.span,
                                },
                            ));
                        }
                    }
                }

                // Check all fields are provided
                for (name, _) in &info.fields {
                    if !provided.contains(name) {
                        return Err(CompileError::new(
                            format!(
                                "missing field '{}' in struct literal for '{}'",
                                name, full_name
                            ),
                            expr.span,
                        ));
                    }
                }

                Ok(TypedExpr {
                    kind: TypedExprKind::StructLit(full_name.clone(), typed_fields),
                    ty: Type::Struct(full_name),
                    span: expr.span,
                })
            }
            ExprKind::EnumVariant(type_name, variant, args) => {
                self.check_enum_variant(type_name, variant, args, None, expr.span)
            }
            ExprKind::QualifiedEnumVariant(module, type_name, variant, args) => {
                self.check_enum_variant(type_name, variant, args, Some(module), expr.span)
            }
            ExprKind::ErrorLit(expr) => {
                let typed = self.check_expr(expr)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::ErrorLit(Box::new(typed.clone())),
                    ty: Type::Error(Box::new(typed.ty)),
                    span: expr.span,
                })
            }
            ExprKind::If(cond, then_block, else_expr) => {
                let typed_cond = self.check_expr(cond)?;

                // Check for type narrowing: if x is T
                // Narrowing is only sound when the variable cannot change between
                // the is-check and the use. A mutable variable that is captured
                // by any closure in the current function can be mutated through
                // the capture at any call site, so narrowing it is unsound.
                let narrowing = if let ExprKind::Is(inner, IsTarget::Type(ty)) = &cond.kind {
                    if let ExprKind::Ident(name) = &inner.kind {
                        let is_mutable = self.lookup_var(name).map(|(_, m)| m).unwrap_or(false);
                        if is_mutable && self.captured_mut_vars.contains(name) {
                            None
                        } else {
                            Some((
                                name.clone(),
                                self.resolve_type_expr(ty)
                                    .map_err(|e| e.with_span(expr.span))?,
                            ))
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if typed_cond.ty != Type::Bool {
                    return Err(CompileError::new(
                        format!(
                            "if condition must be bool, got {}",
                            typed_cond.ty.display_name()
                        ),
                        expr.span,
                    ));
                }

                // Apply narrowing in then block
                if let Some((name, narrow_ty)) = &narrowing {
                    self.push_scope();
                    self.define_var(name, narrow_ty.clone(), false)?;
                }

                let typed_then = self.check_block(then_block)?;

                if narrowing.is_some() {
                    self.pop_scope();
                }

                let typed_else = if let Some(else_expr) = else_expr {
                    // Apply complementary narrowing in else branch
                    let else_narrowing = if let Some((name, narrow_ty)) = &narrowing {
                        let original_ty = self.lookup_var(name).map(|(ty, _)| ty);
                        if let Some(original) = original_ty {
                            let complement = self.subtract_type(&original, narrow_ty);
                            if complement != original {
                                Some((name.clone(), complement))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some((ref name, ref complement_ty)) = else_narrowing {
                        self.push_scope();
                        self.define_var(name, complement_ty.clone(), false)?;
                    }
                    let result = self.check_expr(else_expr)?;
                    if else_narrowing.is_some() {
                        self.pop_scope();
                    }
                    Some(Box::new(result))
                } else {
                    None
                };

                let ty = if let Some(ref else_typed) = typed_else {
                    if self.is_assignable(&typed_then.ty, &else_typed.ty) {
                        else_typed.ty.clone()
                    } else if self.is_assignable(&else_typed.ty, &typed_then.ty) {
                        typed_then.ty.clone()
                    } else if typed_then.ty == Type::Nil || else_typed.ty == Type::Nil {
                        Type::Nil
                    } else {
                        // Different types: compute union
                        let mut members = Vec::new();
                        match &typed_then.ty {
                            Type::Union(m) => members.extend(m.iter().cloned()),
                            other => members.push(other.clone()),
                        }
                        match &else_typed.ty {
                            Type::Union(m) => members.extend(m.iter().cloned()),
                            other => {
                                if !members.contains(other) {
                                    members.push(other.clone());
                                }
                            }
                        }
                        Type::Union(members)
                    }
                } else {
                    Type::Nil
                };

                Ok(TypedExpr {
                    kind: TypedExprKind::If(Box::new(typed_cond), typed_then, typed_else),
                    ty,
                    span: expr.span,
                })
            }
            ExprKind::Match(scrutinee, arms) => {
                let typed_scrutinee = self.check_expr(scrutinee)?;
                let mut typed_arms = Vec::new();
                let mut result_ty: Option<Type> = None;

                for arm in arms {
                    let (bindings, pattern_checked) =
                        self.check_pattern(&arm.pattern, &typed_scrutinee.ty)?;

                    self.push_scope();
                    for (name, ty) in &bindings {
                        self.define_var(name, ty.clone(), false)?;
                    }

                    let typed_body = self.check_expr(&arm.body)?;
                    self.pop_scope();

                    if let Some(ref rty) = result_ty {
                        if !self.is_assignable(&typed_body.ty, rty)
                            && !self.is_assignable(rty, &typed_body.ty)
                        {
                            if typed_body.ty != Type::Nil && *rty != Type::Nil {
                                // Compute union of all arm types
                                let mut members = match rty {
                                    Type::Union(m) => m.clone(),
                                    other => vec![other.clone()],
                                };
                                match &typed_body.ty {
                                    Type::Union(m) => members.extend(m.iter().cloned()),
                                    other => {
                                        if !members.contains(other) {
                                            members.push(other.clone());
                                        }
                                    }
                                }
                                result_ty = Some(Type::Union(members));
                            }
                        } else if self.is_assignable(rty, &typed_body.ty) {
                            // Widen to the more general type
                            result_ty = Some(typed_body.ty.clone());
                        }
                        // If typed_body is assignable to rty, keep rty (it's already general enough)
                    } else {
                        result_ty = Some(typed_body.ty.clone());
                    }

                    typed_arms.push(TypedMatchArm {
                        pattern: pattern_checked,
                        body: typed_body,
                        bindings,
                    });
                }

                // Exhaustiveness check for enums and booleans
                self.check_match_exhaustiveness(&typed_scrutinee.ty, arms, expr.span)?;

                Ok(TypedExpr {
                    kind: TypedExprKind::Match(Box::new(typed_scrutinee), typed_arms),
                    ty: result_ty.unwrap_or(Type::Nil),
                    span: expr.span,
                })
            }
            ExprKind::For(var1, var2, iterable, body) => {
                let typed_iterable = self.check_expr(iterable)?;
                self.push_scope();
                let old_in_loop = self.in_loop;
                self.in_loop = true;

                match &typed_iterable.ty {
                    Type::Array(elem_ty) => {
                        self.define_var(var1, *elem_ty.clone(), false)?;
                        if let Some(var2) = var2 {
                            self.define_var(var2, Type::I64, false)?;
                        }
                    }
                    Type::Map(key_ty, val_ty) => {
                        self.define_var(var1, *key_ty.clone(), false)?;
                        if let Some(var2) = var2 {
                            self.define_var(var2, *val_ty.clone(), false)?;
                        }
                    }
                    Type::String => {
                        self.define_var(var1, Type::String, false)?;
                        if let Some(var2) = var2 {
                            self.define_var(var2, Type::I64, false)?;
                        }
                    }
                    Type::Range => {
                        self.define_var(var1, Type::I64, false)?;
                    }
                    _ => {
                        return Err(CompileError::new(
                            format!("cannot iterate over {}", typed_iterable.ty.display_name()),
                            expr.span,
                        ))
                    }
                }

                let typed_body = self.check_block(body)?;
                self.in_loop = old_in_loop;
                self.pop_scope();

                Ok(TypedExpr {
                    kind: TypedExprKind::For(
                        var1.clone(),
                        var2.clone(),
                        Box::new(typed_iterable),
                        typed_body,
                    ),
                    ty: Type::Nil,
                    span: expr.span,
                })
            }
            ExprKind::While(cond, body) => {
                let typed_cond = self.check_expr(cond)?;
                if typed_cond.ty != Type::Bool {
                    return Err(CompileError::new(
                        format!(
                            "while condition must be bool, got {}",
                            typed_cond.ty.display_name()
                        ),
                        expr.span,
                    ));
                }
                let old_in_loop = self.in_loop;
                self.in_loop = true;
                let typed_body = self.check_block(body)?;
                self.in_loop = old_in_loop;
                Ok(TypedExpr {
                    kind: TypedExprKind::While(Box::new(typed_cond), typed_body),
                    ty: Type::Nil,
                    span: expr.span,
                })
            }
            ExprKind::Loop(body) => {
                let old_in_loop = self.in_loop;
                self.in_loop = true;
                let typed_body = self.check_block(body)?;
                self.in_loop = old_in_loop;
                Ok(TypedExpr {
                    kind: TypedExprKind::Loop(typed_body),
                    ty: Type::Nil,
                    span: expr.span,
                })
            }
            ExprKind::Guard(binding, expr, else_block) => {
                if let Some(name) = binding {
                    // guard let name = expr else { ... }
                    let typed_expr = self.check_expr(expr)?;
                    let typed_else = self.check_block(else_block)?;

                    // The else block MUST diverge (return, break, continue, panic)
                    if !Self::block_diverges(&typed_else) {
                        return Err(CompileError::new(
                            "guard else block must diverge (return, break, continue, or panic)",
                            expr.span,
                        ));
                    }

                    // Determine unwrapped type
                    let unwrapped_ty = match &typed_expr.ty {
                        Type::Optional(inner) => *inner.clone(),
                        Type::Result(ok, _) => *ok.clone(),
                        _ => typed_expr.ty.clone(),
                    };

                    // The binding is available in the enclosing scope (after the guard)
                    self.define_var(name, unwrapped_ty.clone(), false)?;

                    Ok(TypedExpr {
                        kind: TypedExprKind::Guard(
                            Some(name.clone()),
                            Box::new(typed_expr),
                            typed_else,
                        ),
                        ty: Type::Nil,
                        span: expr.span,
                    })
                } else {
                    // guard condition else { ... }
                    let typed_cond = self.check_expr(expr)?;
                    if typed_cond.ty != Type::Bool {
                        return Err(CompileError::new(
                            format!(
                                "guard condition must be bool, got {}",
                                typed_cond.ty.display_name()
                            ),
                            expr.span,
                        ));
                    }
                    let typed_else = self.check_block(else_block)?;

                    // The else block MUST diverge
                    if !Self::block_diverges(&typed_else) {
                        return Err(CompileError::new(
                            "guard else block must diverge (return, break, continue, or panic)",
                            expr.span,
                        ));
                    }

                    Ok(TypedExpr {
                        kind: TypedExprKind::Guard(None, Box::new(typed_cond), typed_else),
                        ty: Type::Nil,
                        span: expr.span,
                    })
                }
            }
            ExprKind::Block(block) => {
                let typed = self.check_block(block)?;
                let ty = typed.ty.clone();
                Ok(TypedExpr {
                    kind: TypedExprKind::Block(typed),
                    ty,
                    span: expr.span,
                })
            }
            ExprKind::Lambda(params, ret_type, body) => {
                let mut param_types = Vec::new();
                self.push_scope();
                for p in params {
                    let ty = self
                        .resolve_type_expr(&p.ty)
                        .map_err(|e| e.with_span(expr.span))?;
                    self.define_var(&p.name, ty.clone(), false)?;
                    param_types.push((p.name.clone(), ty));
                }
                let ret_ty = ret_type
                    .as_ref()
                    .map(|t| self.resolve_type_expr(t))
                    .transpose()?
                    .unwrap_or(Type::Nil);

                let old_return = self.current_fn_return.replace(ret_ty.clone());
                let typed_body = self.check_block(body)?;
                self.current_fn_return = old_return;
                self.pop_scope();

                // Check that non-void lambdas return on all paths
                if ret_ty != Type::Nil
                    && ret_ty != Type::Void
                    && !self.block_always_returns(&typed_body)
                {
                    return Err(CompileError::new(
                        format!(
                            "lambda declares return type {} but not all code paths return a value",
                            ret_ty.display_name()
                        ),
                        expr.span,
                    ));
                }

                let func_type = Type::Function(
                    param_types.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(ret_ty.clone()),
                );
                Ok(TypedExpr {
                    kind: TypedExprKind::Lambda(param_types, ret_ty, typed_body),
                    ty: func_type,
                    span: expr.span,
                })
            }
            ExprKind::As(expr, ty) => {
                let typed = self.check_expr(expr)?;
                let target = self
                    .resolve_type_expr(ty)
                    .map_err(|e| e.with_span(expr.span))?;
                // Validate conversion pair
                self.check_conversion(&typed.ty, &target, false, expr.span)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::As(Box::new(typed), target.clone()),
                    ty: target,
                    span: expr.span,
                })
            }
            ExprKind::AsSafe(expr, ty) => {
                let typed = self.check_expr(expr)?;
                let target = self
                    .resolve_type_expr(ty)
                    .map_err(|e| e.with_span(expr.span))?;
                self.check_conversion(&typed.ty, &target, true, expr.span)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::AsSafe(Box::new(typed), target.clone()),
                    ty: Type::Optional(Box::new(target)),
                    span: expr.span,
                })
            }
            ExprKind::Is(expr, target) => {
                let typed = self.check_expr(expr)?;
                let typed_target = match target {
                    IsTarget::Type(ty) => TypedIsTarget::Type(ty.clone()),
                    IsTarget::EnumVariant(t, v) => TypedIsTarget::EnumVariant(t.clone(), v.clone()),
                    IsTarget::QualifiedVariant(m, t, v) => {
                        TypedIsTarget::QualifiedVariant(m.clone(), t.clone(), v.clone())
                    }
                    IsTarget::Expr(rhs) => {
                        let typed_rhs = self.check_expr(rhs)?;
                        TypedIsTarget::Expr(Box::new(typed_rhs))
                    }
                };
                Ok(TypedExpr {
                    kind: TypedExprKind::Is(Box::new(typed), typed_target),
                    ty: Type::Bool,
                    span: expr.span,
                })
            }
            ExprKind::Try(expr) => {
                let typed = self.check_expr(expr)?;
                match &typed.ty {
                    Type::Result(ok, _) => {
                        let ok_ty = *ok.clone();
                        Ok(TypedExpr {
                            kind: TypedExprKind::Try(Box::new(typed)),
                            ty: ok_ty,
                            span: expr.span,
                        })
                    }
                    _ => Err(CompileError::new(
                        format!(
                            "? operator requires result type, got {}",
                            typed.ty.display_name()
                        ),
                        expr.span,
                    )),
                }
            }
            ExprKind::Range(start, end) => {
                let typed_start = self.check_expr(start)?;
                let typed_end = self.check_expr(end)?;
                if !typed_start.ty.is_integer() {
                    return Err(CompileError::new(
                        format!(
                            "range start must be integer, got {}",
                            typed_start.ty.display_name()
                        ),
                        expr.span,
                    ));
                }
                if !typed_end.ty.is_integer() {
                    return Err(CompileError::new(
                        format!(
                            "range end must be integer, got {}",
                            typed_end.ty.display_name()
                        ),
                        expr.span,
                    ));
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::Range(Box::new(typed_start), Box::new(typed_end)),
                    ty: Type::Range,
                    span: expr.span,
                })
            }
        }
    }

    fn check_call(&mut self, func: &Expr, args: &[Expr]) -> Result<TypedExpr, CompileError> {
        // Handle panic() specially
        if let ExprKind::Ident(name) = &func.kind {
            if name == "panic" {
                let mut typed_args = Vec::new();
                for arg in args {
                    typed_args.push(self.check_expr(arg)?);
                }
                return Ok(TypedExpr {
                    kind: TypedExprKind::Panic(typed_args),
                    ty: Type::Nil, // panic never returns, but for type purposes
                    span: Span::default(),
                });
            }
        }

        let typed_func = self.check_expr(func)?;
        let mut typed_args = Vec::new();
        for arg in args {
            typed_args.push(self.check_expr(arg)?);
        }

        let func_ty = typed_func.ty.clone();
        match &func_ty {
            Type::Function(param_types, ret_type) => {
                if param_types.len() != typed_args.len() {
                    return Err(CompileError::new(
                        format!(
                            "function expects {} arguments, got {}",
                            param_types.len(),
                            typed_args.len()
                        ),
                        func.span,
                    ));
                }
                for (i, (param_ty, arg)) in param_types.iter().zip(typed_args.iter()).enumerate() {
                    if !self.is_assignable(&arg.ty, param_ty) {
                        return Err(CompileError::new(
                            format!(
                                "argument {} type mismatch: expected {}, got {}",
                                i + 1,
                                param_ty.display_name(),
                                arg.ty.display_name()
                            ),
                            func.span,
                        ));
                    }
                }
                let result_ty = *ret_type.clone();
                Ok(TypedExpr {
                    kind: TypedExprKind::Call(Box::new(typed_func), typed_args),
                    ty: result_ty,
                    span: Span::default(),
                })
            }
            _ => Err(CompileError::new(
                format!("cannot call value of type {}", func_ty.display_name()),
                func.span,
            )),
        }
    }


    fn check_method_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<TypedExpr, CompileError> {
        // If obj is a TypeIdent, this is a static method call: Type.method(args)
        if let ExprKind::TypeIdent(type_name) = &obj.kind {
            return self.check_static_method_call(type_name, method, args, obj.span);
        }

        // If obj is an Ident that resolves to a module, treat as static method call
        if let ExprKind::Ident(name) = &obj.kind {
            if self.type_info.modules.contains_key(name.as_str()) {
                return self.check_static_method_call(name, method, args, obj.span);
            }
        }

        // Check mutability for mutating methods
        if is_mutating_method(method) {
            self.check_mutable_target(obj, &format!("call mutating method '{}'", method))?;
        }

        let typed_obj = self.check_expr(obj)?;
        let mut typed_args = Vec::new();
        for arg in args {
            typed_args.push(self.check_expr(arg)?);
        }

        // Check built-in methods
        let result_ty = self.check_builtin_method(&typed_obj.ty, method, &typed_args, obj.span)?;

        Ok(TypedExpr {
            kind: TypedExprKind::MethodCall(Box::new(typed_obj), method.to_string(), typed_args),
            ty: result_ty,
            span: Span::default(),
        })
    }

    fn check_builtin_method(
        &self,
        obj_ty: &Type,
        method: &str,
        args: &[TypedExpr],
        span: Span,
    ) -> Result<Type, CompileError> {
        match obj_ty {
            Type::String => self.check_string_method(method, args, span),
            Type::Array(elem_ty) => self.check_array_method(elem_ty, method, args, span),
            Type::Map(key_ty, val_ty) => self.check_map_method(key_ty, val_ty, method, args, span),
            Type::Struct(name) => {
                // Check user-defined methods
                if let Some(info) = self.type_info.structs.get(name) {
                    if let Some(method_info) = info.methods.get(method) {
                        // Check args (skip self parameter)
                        let expected_args = if method_info.is_method {
                            &method_info.params[1..] // skip self
                        } else {
                            &method_info.params[..]
                        };
                        if expected_args.len() != args.len() {
                            return Err(CompileError::new(
                                format!(
                                    "method '{}' expects {} arguments, got {}",
                                    method,
                                    expected_args.len(),
                                    args.len()
                                ),
                                span,
                            ));
                        }
                        return Ok(method_info.return_type.clone());
                    }
                    // Check for .clone()
                    if method == "clone" && args.is_empty() {
                        return Ok(Type::Struct(name.clone()));
                    }
                }
                // Check module structs
                for mod_info in self.type_info.modules.values() {
                    if let Some(info) = mod_info
                        .structs
                        .get(name.split('.').next_back().unwrap_or(name))
                    {
                        if let Some(method_info) = info.methods.get(method) {
                            return Ok(method_info.return_type.clone());
                        }
                        if method == "clone" && args.is_empty() {
                            return Ok(obj_ty.clone());
                        }
                    }
                }
                // Collect method names from struct and module structs for suggestion
                let mut method_names: Vec<&str> = Vec::new();
                if let Some(info) = self.type_info.structs.get(name) {
                    method_names.extend(info.methods.keys().map(|k| k.as_str()));
                }
                for mod_info in self.type_info.modules.values() {
                    if let Some(info) = mod_info
                        .structs
                        .get(name.split('.').next_back().unwrap_or(name))
                    {
                        method_names.extend(info.methods.keys().map(|k| k.as_str()));
                    }
                }
                let msg = match suggest_similar(method, method_names.into_iter()) {
                    Some(s) => format!(
                        "unknown method '{}' on {}; did you mean '{}'?",
                        method,
                        obj_ty.display_name(),
                        s
                    ),
                    None => format!("unknown method '{}' on {}", method, obj_ty.display_name()),
                };
                Err(CompileError::new(msg, span))
            }
            Type::Enum(name) => {
                if let Some(info) = self.type_info.enums.get(name) {
                    if let Some(method_info) = info.methods.get(method) {
                        return Ok(method_info.return_type.clone());
                    }
                }
                let method_names: Vec<&str> = self
                    .type_info
                    .enums
                    .get(name)
                    .into_iter()
                    .flat_map(|info| info.methods.keys().map(|k| k.as_str()))
                    .collect();
                let msg = match suggest_similar(method, method_names.into_iter()) {
                    Some(s) => format!(
                        "unknown method '{}' on {}; did you mean '{}'?",
                        method,
                        obj_ty.display_name(),
                        s
                    ),
                    None => format!("unknown method '{}' on {}", method, obj_ty.display_name()),
                };
                Err(CompileError::new(msg, span))
            }
            Type::Capability(cap_name) => {
                self.check_capability_method(cap_name, method, args, span)
            }
            _ => Err(CompileError::new(
                format!("unknown method '{}' on {}", method, obj_ty.display_name()),
                span,
            )),
        }
    }

    fn check_string_method(
        &self,
        method: &str,
        args: &[TypedExpr],
        span: Span,
    ) -> Result<Type, CompileError> {
        match method {
            "split" => {
                if args.len() != 1 {
                    return Err(CompileError::new("split takes 1 argument", span));
                }
                Ok(Type::Array(Box::new(Type::String)))
            }
            "trim" | "trim_start" | "trim_end" | "upper" | "lower" => {
                if !args.is_empty() {
                    return Err(CompileError::new(
                        format!("{} takes 0 arguments", method),
                        span,
                    ));
                }
                Ok(Type::String)
            }
            "replace" => {
                if args.len() != 2 {
                    return Err(CompileError::new("replace takes 2 arguments", span));
                }
                Ok(Type::String)
            }
            "find" => {
                if args.len() != 1 {
                    return Err(CompileError::new("find takes 1 argument", span));
                }
                Ok(Type::Optional(Box::new(Type::I64)))
            }
            "substring" => {
                if args.len() != 2 {
                    return Err(CompileError::new("substring takes 2 arguments", span));
                }
                Ok(Type::String)
            }
            "starts_with" | "ends_with" | "contains" => {
                if args.len() != 1 {
                    return Err(CompileError::new(
                        format!("{} takes 1 argument", method),
                        span,
                    ));
                }
                Ok(Type::Bool)
            }
            "repeat" => {
                if args.len() != 1 {
                    return Err(CompileError::new("repeat takes 1 argument", span));
                }
                Ok(Type::String)
            }
            "char_at" => {
                if args.len() != 1 {
                    return Err(CompileError::new("char_at takes 1 argument", span));
                }
                Ok(Type::String)
            }
            "index_of" | "last_index_of" => {
                if args.len() != 1 {
                    return Err(CompileError::new(
                        format!("{} takes 1 argument", method),
                        span,
                    ));
                }
                Ok(Type::I64)
            }
            "slice" => {
                if args.len() != 2 {
                    return Err(CompileError::new("slice takes 2 arguments", span));
                }
                Ok(Type::String)
            }
            "chars" => {
                if !args.is_empty() {
                    return Err(CompileError::new("chars takes 0 arguments", span));
                }
                Ok(Type::Array(Box::new(Type::String)))
            }
            "bytes" => {
                if !args.is_empty() {
                    return Err(CompileError::new("bytes takes 0 arguments", span));
                }
                Ok(Type::Array(Box::new(Type::I64)))
            }
            _ => Err(CompileError::new(
                format!("unknown string method '{}'", method),
                span,
            )),
        }
    }

    fn check_array_method(
        &self,
        elem_ty: &Type,
        method: &str,
        args: &[TypedExpr],
        span: Span,
    ) -> Result<Type, CompileError> {
        match method {
            "push" => {
                if args.len() != 1 {
                    return Err(CompileError::new("push takes 1 argument", span));
                }
                if !self.is_assignable(&args[0].ty, elem_ty) {
                    return Err(CompileError::new(
                        format!(
                            "push type mismatch: array is [{}], got {}",
                            elem_ty.display_name(),
                            args[0].ty.display_name()
                        ),
                        span,
                    ));
                }
                Ok(Type::Nil)
            }
            "pop" => {
                if !args.is_empty() {
                    return Err(CompileError::new("pop takes 0 arguments", span));
                }
                Ok(elem_ty.clone())
            }
            "insert" => {
                if args.len() != 2 {
                    return Err(CompileError::new("insert takes 2 arguments", span));
                }
                Ok(Type::Nil)
            }
            "remove" => {
                if args.len() != 1 {
                    return Err(CompileError::new("remove takes 1 argument", span));
                }
                Ok(Type::Nil)
            }
            "sort" | "reverse" => {
                if !args.is_empty() {
                    return Err(CompileError::new(
                        format!("{} takes 0 arguments", method),
                        span,
                    ));
                }
                Ok(Type::Nil)
            }
            "join" => {
                if args.len() != 1 {
                    return Err(CompileError::new("join takes 1 argument", span));
                }
                Ok(Type::String)
            }
            "contains" => {
                if args.len() != 1 {
                    return Err(CompileError::new("contains takes 1 argument", span));
                }
                Ok(Type::Bool)
            }
            "clone" => {
                if !args.is_empty() {
                    return Err(CompileError::new("clone takes 0 arguments", span));
                }
                Ok(Type::Array(Box::new(elem_ty.clone())))
            }
            "map" => {
                if args.len() != 1 {
                    return Err(CompileError::new("map takes 1 argument", span));
                }
                if let Type::Function(params, ret) = &args[0].ty {
                    if params.len() != 1 {
                        return Err(CompileError::new("map callback must take 1 argument", span));
                    }
                    Ok(Type::Array(ret.clone()))
                } else {
                    Err(CompileError::new("map argument must be a function", span))
                }
            }
            "filter" => {
                if args.len() != 1 {
                    return Err(CompileError::new("filter takes 1 argument", span));
                }
                if let Type::Function(params, ret) = &args[0].ty {
                    if params.len() != 1 {
                        return Err(CompileError::new(
                            "filter callback must take 1 argument",
                            span,
                        ));
                    }
                    if **ret != Type::Bool {
                        return Err(CompileError::new("filter callback must return bool", span));
                    }
                    Ok(Type::Array(Box::new(elem_ty.clone())))
                } else {
                    Err(CompileError::new(
                        "filter argument must be a function",
                        span,
                    ))
                }
            }
            "find" => {
                if args.len() != 1 {
                    return Err(CompileError::new("find takes 1 argument", span));
                }
                if let Type::Function(params, ret) = &args[0].ty {
                    if params.len() != 1 {
                        return Err(CompileError::new(
                            "find callback must take 1 argument",
                            span,
                        ));
                    }
                    if **ret != Type::Bool {
                        return Err(CompileError::new("find callback must return bool", span));
                    }
                    Ok(Type::Optional(Box::new(elem_ty.clone())))
                } else {
                    Err(CompileError::new("find argument must be a function", span))
                }
            }
            "any" | "all" => {
                if args.len() != 1 {
                    return Err(CompileError::new(
                        format!("{} takes 1 argument", method),
                        span,
                    ));
                }
                if let Type::Function(params, ret) = &args[0].ty {
                    if params.len() != 1 {
                        return Err(CompileError::new(
                            format!("{} callback must take 1 argument", method),
                            span,
                        ));
                    }
                    if **ret != Type::Bool {
                        return Err(CompileError::new(
                            format!("{} callback must return bool", method),
                            span,
                        ));
                    }
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new(
                        format!("{} argument must be a function", method),
                        span,
                    ))
                }
            }
            _ => Err(CompileError::new(
                format!("unknown array method '{}'", method),
                span,
            )),
        }
    }

    fn check_map_method(
        &self,
        key_ty: &Type,
        val_ty: &Type,
        method: &str,
        args: &[TypedExpr],
        span: Span,
    ) -> Result<Type, CompileError> {
        match method {
            "get" => {
                if args.len() != 1 {
                    return Err(CompileError::new("get takes 1 argument", span));
                }
                Ok(Type::Optional(Box::new(val_ty.clone())))
            }
            "contains_key" => {
                if args.len() != 1 {
                    return Err(CompileError::new("contains_key takes 1 argument", span));
                }
                Ok(Type::Bool)
            }
            "keys" => {
                if !args.is_empty() {
                    return Err(CompileError::new("keys takes 0 arguments", span));
                }
                Ok(Type::Array(Box::new(key_ty.clone())))
            }
            "values" => {
                if !args.is_empty() {
                    return Err(CompileError::new("values takes 0 arguments", span));
                }
                Ok(Type::Array(Box::new(val_ty.clone())))
            }
            "remove" => {
                if args.len() != 1 {
                    return Err(CompileError::new("remove takes 1 argument", span));
                }
                Ok(Type::Nil)
            }
            "clone" => {
                if !args.is_empty() {
                    return Err(CompileError::new("clone takes 0 arguments", span));
                }
                Ok(Type::Map(
                    Box::new(key_ty.clone()),
                    Box::new(val_ty.clone()),
                ))
            }
            _ => Err(CompileError::new(
                format!("unknown map method '{}'", method),
                span,
            )),
        }
    }

    fn check_capability_method(
        &self,
        cap: &str,
        method: &str,
        args: &[TypedExpr],
        span: Span,
    ) -> Result<Type, CompileError> {
        match cap {
            "Stdout" => match method {
                "println" | "print" => {
                    if args.len() != 1 {
                        return Err(CompileError::new(
                            format!("{} takes 1 argument", method),
                            span,
                        ));
                    }
                    Ok(Type::Nil)
                }
                "flush" => {
                    if !args.is_empty() {
                        return Err(CompileError::new("flush takes 0 arguments", span));
                    }
                    Ok(Type::Nil)
                }
                _ => Err(CompileError::new(
                    format!("unknown method '{}' on Stdout", method),
                    span,
                )),
            },
            "Stdin" => match method {
                "read_line" => {
                    if !args.is_empty() {
                        return Err(CompileError::new("read_line takes 0 arguments", span));
                    }
                    Ok(Type::String)
                }
                "read_key" => {
                    if !args.is_empty() {
                        return Err(CompileError::new("read_key takes 0 arguments", span));
                    }
                    Ok(Type::String)
                }
                _ => Err(CompileError::new(
                    format!("unknown method '{}' on Stdin", method),
                    span,
                )),
            },
            "Fs" => match method {
                "read_file" => {
                    if args.len() != 1 {
                        return Err(CompileError::new("read_file takes 1 argument", span));
                    }
                    Ok(Type::Result(Box::new(Type::String), Box::new(Type::String)))
                }
                "write_file" => {
                    if args.len() != 2 {
                        return Err(CompileError::new("write_file takes 2 arguments", span));
                    }
                    Ok(Type::Result(Box::new(Type::Nil), Box::new(Type::String)))
                }
                "read_dir" => {
                    if args.len() != 1 {
                        return Err(CompileError::new("read_dir takes 1 argument", span));
                    }
                    Ok(Type::Result(
                        Box::new(Type::Array(Box::new(Type::String))),
                        Box::new(Type::String),
                    ))
                }
                "exists" | "is_dir" => {
                    if args.len() != 1 {
                        return Err(CompileError::new(
                            format!("{} takes 1 argument", method),
                            span,
                        ));
                    }
                    Ok(Type::Bool)
                }
                "mkdir" => {
                    if args.len() != 1 {
                        return Err(CompileError::new("mkdir takes 1 argument", span));
                    }
                    Ok(Type::Result(Box::new(Type::Nil), Box::new(Type::String)))
                }
                "join" => {
                    if args.is_empty() {
                        return Err(CompileError::new("join takes at least 1 argument", span));
                    }
                    Ok(Type::String)
                }
                "extension" | "stem" => {
                    if args.len() != 1 {
                        return Err(CompileError::new(
                            format!("{} takes 1 argument", method),
                            span,
                        ));
                    }
                    Ok(Type::Optional(Box::new(Type::String)))
                }
                _ => Err(CompileError::new(
                    format!("unknown method '{}' on Fs", method),
                    span,
                )),
            },
            "Env" => match method {
                "args" => {
                    if !args.is_empty() {
                        return Err(CompileError::new("args takes 0 arguments", span));
                    }
                    Ok(Type::Array(Box::new(Type::String)))
                }
                "get" => {
                    if args.len() != 1 {
                        return Err(CompileError::new("get takes 1 argument", span));
                    }
                    Ok(Type::Optional(Box::new(Type::String)))
                }
                _ => Err(CompileError::new(
                    format!("unknown method '{}' on Env", method),
                    span,
                )),
            },
            "Highlight" => match method {
                "code" | "inline" => {
                    if args.len() != 2 {
                        return Err(CompileError::new(
                            format!("{} takes 2 arguments", method),
                            span,
                        ));
                    }
                    Ok(Type::String)
                }
                _ => Err(CompileError::new(
                    format!("unknown method '{}' on Highlight", method),
                    span,
                )),
            },
            _ => Err(CompileError::new(
                format!("unknown capability '{}'", cap),
                span,
            )),
        }
    }

    fn check_static_method_call(
        &mut self,
        name: &str,
        method: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, CompileError> {
        let mut typed_args = Vec::new();
        for arg in args {
            typed_args.push(self.check_expr(arg)?);
        }

        // Note: define_var rejects names that collide with module names,
        // so a local variable with the same name as a module cannot exist.

        // Check if name is a module
        if let Some(mod_info) = self.type_info.modules.get(name).cloned() {
            // module.function(args)
            if let Some(func_info) = mod_info.functions.get(method) {
                if func_info.params.len() != typed_args.len() {
                    return Err(CompileError::new(
                        format!(
                            "function '{}.{}' expects {} arguments, got {}",
                            name,
                            method,
                            func_info.params.len(),
                            typed_args.len()
                        ),
                        span,
                    ));
                }
                return Ok(TypedExpr {
                    kind: TypedExprKind::StaticMethodCall(
                        name.to_string(),
                        method.to_string(),
                        typed_args,
                    ),
                    ty: func_info.return_type.clone(),
                    span: Span::default(),
                });
            }
            // Could be module.Type(args) - enum variant with single field
            if let Some(_enum_info) = mod_info.enums.get(method) {
                // This is actually accessing a type, not calling a function
                return Err(CompileError::new(
                    format!("'{}' is a type, not a function", method),
                    span,
                ));
            }
        }

        // Check if name is a type with a static method
        if let Some(struct_info) = self.type_info.structs.get(name).cloned() {
            if let Some(method_info) = struct_info.methods.get(method) {
                if !method_info.is_method {
                    // Static method
                    if method_info.params.len() != typed_args.len() {
                        return Err(CompileError::new(
                            format!(
                                "static method '{}.{}' expects {} arguments, got {}",
                                name,
                                method,
                                method_info.params.len(),
                                typed_args.len()
                            ),
                            span,
                        ));
                    }
                    return Ok(TypedExpr {
                        kind: TypedExprKind::StaticMethodCall(
                            name.to_string(),
                            method.to_string(),
                            typed_args,
                        ),
                        ty: method_info.return_type.clone(),
                        span: Span::default(),
                    });
                }
            }
        }

        // Check for enum variant construction that looks like a static method call
        if let Some(enum_info) = self.type_info.enums.get(name).cloned() {
            if let Some(variant) = enum_info.variants.iter().find(|v| v.name == method) {
                if variant.fields.len() != typed_args.len() {
                    return Err(CompileError::new(
                        format!(
                            "enum variant '{}.{}' expects {} arguments, got {}",
                            name,
                            method,
                            variant.fields.len(),
                            typed_args.len()
                        ),
                        span,
                    ));
                }
                return Ok(TypedExpr {
                    kind: TypedExprKind::EnumVariant(
                        name.to_string(),
                        method.to_string(),
                        typed_args,
                    ),
                    ty: Type::Enum(name.to_string()),
                    span: Span::default(),
                });
            }
        }

        Err(CompileError::new(
            format!("unknown function or method '{}.{}'", name, method),
            span,
        ))
    }

    fn check_enum_variant(
        &mut self,
        type_name: &str,
        variant: &str,
        args: &[Expr],
        module: Option<&String>,
        span: Span,
    ) -> Result<TypedExpr, CompileError> {
        let full_name = if let Some(m) = module {
            format!("{}.{}", m, type_name)
        } else {
            type_name.to_string()
        };

        let enum_info = if let Some(m) = module {
            self.type_info
                .modules
                .get(m.as_str())
                .and_then(|mi| mi.enums.get(type_name))
                .cloned()
        } else {
            self.type_info.enums.get(type_name).cloned()
        };

        let info = enum_info
            .ok_or_else(|| CompileError::new(format!("unknown enum type '{}'", full_name), span))?;

        let variant_info = info
            .variants
            .iter()
            .find(|v| v.name == variant)
            .ok_or_else(|| {
                CompileError::new(format!("unknown variant '{}.{}'", full_name, variant), span)
            })?;

        if variant_info.fields.len() != args.len() {
            return Err(CompileError::new(
                format!(
                    "variant '{}.{}' expects {} arguments, got {}",
                    full_name,
                    variant,
                    variant_info.fields.len(),
                    args.len()
                ),
                span,
            ));
        }

        let mut typed_args = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let typed = self.check_expr(arg)?;
            if !self.is_assignable(&typed.ty, &variant_info.fields[i]) {
                return Err(CompileError::new(
                    format!(
                        "variant field type mismatch: expected {}, got {}",
                        variant_info.fields[i].display_name(),
                        typed.ty.display_name()
                    ),
                    span,
                ));
            }
            typed_args.push(typed);
        }

        Ok(TypedExpr {
            kind: TypedExprKind::EnumVariant(full_name.clone(), variant.to_string(), typed_args),
            ty: Type::Enum(full_name),
            span: Span::default(),
        })
    }

    fn check_binop(
        &self,
        left: &Type,
        op: &BinOp,
        right: &Type,
        span: Span,
    ) -> Result<Type, CompileError> {
        match op {
            BinOp::Add => {
                // String concatenation
                if *left == Type::String && *right == Type::String {
                    return Ok(Type::String);
                }
                // Numeric addition
                if left == right && left.is_numeric() {
                    return Ok(left.clone());
                }
                Err(CompileError::new(
                    format!(
                        "cannot add {} and {}",
                        left.display_name(),
                        right.display_name()
                    ),
                    span,
                ))
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if left == right && left.is_numeric() {
                    return Ok(left.clone());
                }
                Err(CompileError::new(
                    format!(
                        "cannot apply {:?} to {} and {}",
                        op,
                        left.display_name(),
                        right.display_name()
                    ),
                    span,
                ))
            }
            BinOp::Pow => {
                if left == right && left.is_numeric() {
                    return Ok(left.clone());
                }
                Err(CompileError::new(
                    format!(
                        "cannot apply ** to {} and {}",
                        left.display_name(),
                        right.display_name()
                    ),
                    span,
                ))
            }
            BinOp::Eq | BinOp::NotEq => {
                if left == right || (left == &Type::Nil || right == &Type::Nil) {
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new(
                        format!(
                            "cannot compare {} and {} for equality",
                            left.display_name(),
                            right.display_name()
                        ),
                        span,
                    ))
                }
            }
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if left == right && (left.is_numeric() || *left == Type::String) {
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new(
                        format!(
                            "cannot compare {} and {}",
                            left.display_name(),
                            right.display_name()
                        ),
                        span,
                    ))
                }
            }
            BinOp::And | BinOp::Or => {
                if *left == Type::Bool && *right == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new(
                        format!(
                            "logical operators require bool operands, got {} and {}",
                            left.display_name(),
                            right.display_name()
                        ),
                        span,
                    ))
                }
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::ShiftLeft | BinOp::ShiftRight => {
                if left == right && left.is_integer() {
                    Ok(left.clone())
                } else {
                    Err(CompileError::new(
                        format!(
                            "bitwise operators require matching integer types, got {} and {}",
                            left.display_name(),
                            right.display_name()
                        ),
                        span,
                    ))
                }
            }
        }
    }

    fn check_conversion(
        &self,
        from: &Type,
        to: &Type,
        _is_safe: bool,
        span: Span,
    ) -> Result<(), CompileError> {
        // Same type (no-op cast, can arise after type narrowing)
        if from == to {
            return Ok(());
        }
        // Integer to integer
        if from.is_integer() && to.is_integer() {
            return Ok(());
        }
        // Integer to f64
        if from.is_integer() && *to == Type::F64 {
            return Ok(());
        }
        // f64 to integer
        if *from == Type::F64 && to.is_integer() {
            return Ok(());
        }
        // Numeric to string
        if from.is_numeric() && *to == Type::String {
            return Ok(());
        }
        // String to numeric
        if *from == Type::String && to.is_numeric() {
            return Ok(());
        }
        // Bool to string
        if *from == Type::Bool && *to == Type::String {
            return Ok(());
        }
        // nil to string
        if *from == Type::Nil && *to == Type::String {
            return Ok(());
        }
        // Optional to string (displays "nil" or the inner value)
        if matches!(from, Type::Optional(_)) && *to == Type::String {
            return Ok(());
        }
        // Optional unwrap: T? as T (panics on nil)
        if let Type::Optional(inner) = from {
            if self.is_assignable(inner, to) {
                return Ok(());
            }
        }
        // Enum/Struct/Error to string (display representation)
        if matches!(
            from,
            Type::Enum(_) | Type::Struct(_) | Type::Error(_) | Type::Result(_, _)
        ) && *to == Type::String
        {
            return Ok(());
        }

        Err(CompileError::new(
            format!(
                "cannot convert {} to {}",
                from.display_name(),
                to.display_name()
            ),
            span,
        ))
    }

    fn get_field_type(&self, ty: &Type, field: &str, span: Span) -> Result<Type, CompileError> {
        match ty {
            Type::Struct(name) => {
                // Check local structs
                if let Some(info) = self.type_info.structs.get(name) {
                    for (fname, fty) in &info.fields {
                        if fname == field {
                            return Ok(fty.clone());
                        }
                    }
                }
                // Check module structs
                if name.contains('.') {
                    let parts: Vec<&str> = name.splitn(2, '.').collect();
                    if let Some(mod_info) = self.type_info.modules.get(parts[0]) {
                        if let Some(struct_info) = mod_info.structs.get(parts[1]) {
                            for (fname, fty) in &struct_info.fields {
                                if fname == field {
                                    return Ok(fty.clone());
                                }
                            }
                        }
                    }
                }
                // Collect all known fields for suggestion
                let all_fields: Vec<&str> = self
                    .type_info
                    .structs
                    .get(name)
                    .into_iter()
                    .flat_map(|info| info.fields.iter().map(|(n, _)| n.as_str()))
                    .collect();
                let msg = match suggest_similar(field, all_fields.into_iter()) {
                    Some(s) => format!(
                        "unknown field '{}' on {}; did you mean '{}'?",
                        field, name, s
                    ),
                    None => format!("unknown field '{}' on {}", field, name),
                };
                Err(CompileError::new(msg, span))
            }
            _ => Err(CompileError::new(
                format!("cannot access field '{}' on {}", field, ty.display_name()),
                span,
            )),
        }
    }

    fn get_index_type(
        &self,
        obj_ty: &Type,
        idx_ty: &Type,
        span: Span,
    ) -> Result<Type, CompileError> {
        match obj_ty {
            Type::Array(elem_ty) => {
                if !idx_ty.is_integer() {
                    return Err(CompileError::new(
                        format!("array index must be integer, got {}", idx_ty.display_name()),
                        span,
                    ));
                }
                Ok(*elem_ty.clone())
            }
            Type::Map(key_ty, val_ty) => {
                if !self.is_assignable(idx_ty, key_ty) {
                    return Err(CompileError::new(
                        format!(
                            "map key type mismatch: expected {}, got {}",
                            key_ty.display_name(),
                            idx_ty.display_name()
                        ),
                        span,
                    ));
                }
                Ok(*val_ty.clone())
            }
            _ => Err(CompileError::new(
                format!("cannot index into {}", obj_ty.display_name()),
                span,
            )),
        }
    }

    /// Check that the root variable of an expression is mutable.
    /// Returns Ok(()) if mutable or if the expression has no named root (e.g. function return).
    /// Returns Err if the root variable is immutable.
    fn check_mutable_target(&self, expr: &Expr, action: &str) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Ident(name) => {
                if let Some((_, mutable)) = self.lookup_var(name) {
                    if !mutable {
                        return Err(CompileError::new(
                            format!(
                                "cannot {} on immutable binding '{}' (declare with 'let mut')",
                                action, name
                            ),
                            expr.span,
                        ));
                    }
                }
                Ok(())
            }
            ExprKind::FieldAccess(inner, _) | ExprKind::Index(inner, _) => {
                self.check_mutable_target(inner, action)
            }
            // For method call returns, function returns, etc. — allow mutation
            // (the value is a temporary, not bound to any variable)
            _ => Ok(()),
        }
    }

    /// Extract an integer literal value, including through unary negation.
    fn extract_int_literal(expr: &TypedExpr) -> Option<i128> {
        match &expr.kind {
            TypedExprKind::IntLit(n) => Some(*n),
            TypedExprKind::UnaryOp(crate::ast::UnaryOp::Neg, inner) => {
                if let TypedExprKind::IntLit(n) = &inner.kind {
                    n.checked_neg()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check that an integer literal fits within a declared narrow integer type.
    fn check_int_literal_range(
        n: i128,
        ty: &Type,
        name: &str,
        span: Span,
    ) -> Result<(), CompileError> {
        let (min, max): (i128, i128) = match ty {
            Type::I8 => (i8::MIN as i128, i8::MAX as i128),
            Type::I16 => (i16::MIN as i128, i16::MAX as i128),
            Type::I32 => (i32::MIN as i128, i32::MAX as i128),
            Type::I64 => (i64::MIN as i128, i64::MAX as i128),
            Type::U8 => (0, u8::MAX as i128),
            Type::U16 => (0, u16::MAX as i128),
            Type::U32 => (0, u32::MAX as i128),
            Type::U64 => (0, u64::MAX as i128),
            _ => return Ok(()), // non-integer — no check needed
        };
        if n < min || n > max {
            return Err(CompileError::new(
                format!(
                    "integer literal {} out of range for {} in '{}'",
                    n,
                    ty.display_name(),
                    name
                ),
                span,
            ));
        }
        Ok(())
    }

    /// Check whether a block always diverges (return, break, continue, panic).
    fn block_diverges(block: &TypedBlock) -> bool {
        if let Some(last) = block.stmts.last() {
            match last {
                TypedStmt::Return(_)
                | TypedStmt::ReturnError(_)
                | TypedStmt::Break
                | TypedStmt::Continue => true,
                TypedStmt::Expr(expr) => matches!(&expr.kind, TypedExprKind::Panic(_)),
                _ => false,
            }
        } else {
            false
        }
    }

    fn check_match_exhaustiveness(
        &self,
        scrutinee_ty: &Type,
        arms: &[MatchArm],
        span: Span,
    ) -> Result<(), CompileError> {
        // Check if any arm is a catch-all (wildcard or binding)
        let has_catch_all = arms.iter().any(|arm| {
            matches!(
                arm.pattern.kind,
                PatternKind::Wildcard | PatternKind::Binding(_)
            )
        });
        if has_catch_all {
            return Ok(());
        }

        match scrutinee_ty {
            Type::Enum(name) => {
                let enum_info = if let Some(info) = self.type_info.enums.get(name) {
                    info
                } else {
                    return Ok(()); // Unknown enum, skip check
                };
                let variant_count = enum_info.variants.len();
                let mut covered = vec![false; variant_count];
                for arm in arms {
                    match &arm.pattern.kind {
                        PatternKind::EnumVariant(_, variant, _)
                        | PatternKind::QualifiedEnumVariant(_, _, variant, _) => {
                            if let Some(idx) =
                                enum_info.variants.iter().position(|v| v.name == *variant)
                            {
                                covered[idx] = true;
                            }
                        }
                        PatternKind::IsEnumVariant(_, variant) => {
                            if let Some(idx) =
                                enum_info.variants.iter().position(|v| v.name == *variant)
                            {
                                covered[idx] = true;
                            }
                        }
                        _ => {}
                    }
                }
                let missing: Vec<&str> = enum_info
                    .variants
                    .iter()
                    .zip(covered.iter())
                    .filter(|(_, &c)| !c)
                    .map(|(v, _)| v.name.as_str())
                    .collect();
                if !missing.is_empty() {
                    return Err(CompileError::new(
                        format!(
                            "non-exhaustive match on '{}': missing variant(s) {}",
                            name,
                            missing.join(", ")
                        ),
                        span,
                    ));
                }
            }
            Type::Bool => {
                let mut has_true = false;
                let mut has_false = false;
                for arm in arms {
                    match &arm.pattern.kind {
                        PatternKind::BoolLit(true) => has_true = true,
                        PatternKind::BoolLit(false) => has_false = true,
                        _ => {}
                    }
                }
                if !has_true || !has_false {
                    return Err(CompileError::new(
                        "non-exhaustive match on bool: missing true or false branch",
                        span,
                    ));
                }
            }
            Type::Union(members) => {
                let mut covered = vec![false; members.len()];
                for arm in arms {
                    if let PatternKind::IsType(ty_expr) = &arm.pattern.kind {
                        if let Ok(resolved) = self.resolve_type_expr(ty_expr) {
                            for (i, member) in members.iter().enumerate() {
                                if *member == resolved {
                                    covered[i] = true;
                                }
                            }
                        }
                    }
                }
                let missing: Vec<String> = members
                    .iter()
                    .zip(covered.iter())
                    .filter(|(_, &c)| !c)
                    .map(|(t, _)| t.display_name())
                    .collect();
                if !missing.is_empty() {
                    return Err(CompileError::new(
                        format!(
                            "non-exhaustive match on union type: missing type(s) {}",
                            missing.join(", ")
                        ),
                        span,
                    ));
                }
            }
            // For integers, strings, floats: require a wildcard/binding pattern
            // since we can't enumerate all possible values.
            _ => {
                let has_wildcard = arms.iter().any(|arm| {
                    matches!(
                        arm.pattern.kind,
                        PatternKind::Wildcard | PatternKind::Binding(_)
                    )
                });
                if !has_wildcard && !arms.is_empty() {
                    return Err(CompileError::new(
                        format!(
                            "non-exhaustive match on '{}': add a wildcard (_) or binding pattern to handle all values",
                            scrutinee_ty.display_name()
                        ),
                        span,
                    ));
                }
                if arms.is_empty() {
                    return Err(CompileError::new("empty match expression", span));
                }
            }
        }
        Ok(())
    }

    fn check_pattern(
        &self,
        pattern: &Pattern,
        scrutinee_ty: &Type,
    ) -> Result<(Vec<(String, Type)>, Pattern), CompileError> {
        let mut bindings = Vec::new();

        match &pattern.kind {
            PatternKind::Wildcard => {}
            PatternKind::IntLit(_)
            | PatternKind::FloatLit(_)
            | PatternKind::BoolLit(_)
            | PatternKind::StringLit(_)
            | PatternKind::Nil => {}
            PatternKind::Binding(name) => {
                if name != "_" {
                    // For result types, a non-error binding gets the success type
                    let ty = match scrutinee_ty {
                        Type::Result(ok, _) => *ok.clone(),
                        Type::Optional(inner) => *inner.clone(),
                        _ => scrutinee_ty.clone(),
                    };
                    bindings.push((name.clone(), ty));
                }
            }
            PatternKind::EnumVariant(type_name, variant, bound_names) => {
                if let Some(info) = self.type_info.enums.get(type_name) {
                    if let Some(v) = info.variants.iter().find(|v| v.name == *variant) {
                        if bound_names.len() != v.fields.len() {
                            return Err(CompileError::new(
                                format!(
                                    "pattern for '{}.{}' expects {} bindings, got {}",
                                    type_name,
                                    variant,
                                    v.fields.len(),
                                    bound_names.len()
                                ),
                                pattern.span,
                            ));
                        }
                        for (i, name) in bound_names.iter().enumerate() {
                            if name != "_" {
                                bindings.push((name.clone(), v.fields[i].clone()));
                            }
                        }
                    }
                }
            }
            PatternKind::QualifiedEnumVariant(module, type_name, variant, bound_names) => {
                if let Some(mod_info) = self.type_info.modules.get(module.as_str()) {
                    if let Some(info) = mod_info.enums.get(type_name.as_str()) {
                        if let Some(v) = info.variants.iter().find(|v| v.name == *variant) {
                            for (i, name) in bound_names.iter().enumerate() {
                                if name != "_" && i < v.fields.len() {
                                    bindings.push((name.clone(), v.fields[i].clone()));
                                }
                            }
                        }
                    }
                }
            }
            PatternKind::Error(name) => {
                // For result types, bind the error value
                match scrutinee_ty {
                    Type::Result(_, err_ty) => {
                        bindings.push((name.clone(), *err_ty.clone()));
                    }
                    _ => {
                        bindings.push((name.clone(), Type::String));
                    }
                }
            }
            PatternKind::IsType(ty) => {
                // Type narrowing in match
                if let Ok(_resolved) = self.resolve_type_expr(ty) {
                    // No new binding, but the scrutinee gets narrowed
                }
            }
            PatternKind::IsEnumVariant(_, _) => {}
        }

        Ok((bindings, pattern.clone()))
    }

    /// Check if a block always returns a value (for missing-return detection).
    /// Conservative: returns true only if every path through the block ends in return.
    fn block_always_returns(&self, block: &TypedBlock) -> bool {
        if block.stmts.is_empty() {
            return false;
        }
        match block.stmts.last() {
            Some(TypedStmt::Return(Some(_))) => true,
            Some(TypedStmt::ReturnError(_)) => true,
            Some(TypedStmt::Expr(expr)) => match &expr.kind {
                // if/else where both branches return
                TypedExprKind::If(_, then_block, Some(else_expr)) => {
                    self.block_always_returns(then_block) && self.expr_always_returns(else_expr)
                }
                TypedExprKind::Match(_, arms) => {
                    !arms.is_empty() && arms.iter().all(|arm| self.expr_always_returns(&arm.body))
                }
                TypedExprKind::Block(inner) => self.block_always_returns(inner),
                TypedExprKind::Loop(_) => true, // loop without break runs forever
                _ => false,
            },
            _ => false,
        }
    }

    fn expr_always_returns(&self, expr: &TypedExpr) -> bool {
        match &expr.kind {
            TypedExprKind::Block(block) => self.block_always_returns(block),
            TypedExprKind::If(_, then_block, Some(else_expr)) => {
                self.block_always_returns(then_block) && self.expr_always_returns(else_expr)
            }
            TypedExprKind::Match(_, arms) => {
                !arms.is_empty() && arms.iter().all(|arm| self.expr_always_returns(&arm.body))
            }
            _ => false,
        }
    }
}

/// Methods that mutate the receiver object in place.
fn is_mutating_method(name: &str) -> bool {
    matches!(
        name,
        "push" | "pop" | "insert" | "remove" | "sort" | "reverse"
    )
}

/// Find mutable variables from outer scopes that are referenced inside lambdas
/// in the given block. These variables are unsafe to narrow because any function
/// call could invoke the capturing closure and mutate the variable.
fn find_captured_mut_vars(
    block: &Block,
    scopes: &[HashMap<String, (Type, bool)>],
) -> HashSet<String> {
    let mut result = HashSet::new();
    // Collect mutable variable names from current scopes
    let mut mutable_vars = HashSet::new();
    for scope in scopes {
        for (name, (_, is_mut)) in scope {
            if *is_mut {
                mutable_vars.insert(name.clone());
            }
        }
    }
    // Also collect let-mut declarations inside the block itself,
    // since those aren't in scopes yet at scan time
    collect_mut_declarations(block, &mut mutable_vars);
    // Walk the block looking for lambdas that capture those mutable vars
    for stmt in &block.stmts {
        find_lambdas_in_stmt(&stmt.kind, &mutable_vars, &mut result);
    }
    result
}

fn collect_mut_declarations(block: &Block, mutable_vars: &mut HashSet<String>) {
    for stmt in &block.stmts {
        if let StmtKind::Let(decl) = &stmt.kind {
            if decl.mutable {
                mutable_vars.insert(decl.name.clone());
            }
        }
        // Recurse into nested blocks (if, for, while, etc.)
        if let StmtKind::Expr(e) = &stmt.kind {
            collect_mut_decls_in_expr(&e.kind, mutable_vars);
        }
    }
}

fn collect_mut_decls_in_expr(expr: &ExprKind, mutable_vars: &mut HashSet<String>) {
    match expr {
        ExprKind::If(_, then_block, else_expr) => {
            collect_mut_declarations(then_block, mutable_vars);
            if let Some(e) = else_expr {
                collect_mut_decls_in_expr(&e.kind, mutable_vars);
            }
        }
        ExprKind::For(_, _, _, body)
        | ExprKind::While(_, body)
        | ExprKind::Loop(body)
        | ExprKind::Block(body) => {
            collect_mut_declarations(body, mutable_vars);
        }
        _ => {}
    }
}

fn find_lambdas_in_stmt(
    stmt: &StmtKind,
    mutable_vars: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    match stmt {
        StmtKind::Let(decl) => find_lambdas_in_expr(&decl.value.kind, mutable_vars, captured),
        StmtKind::LetTuple(_, expr) => find_lambdas_in_expr(&expr.kind, mutable_vars, captured),
        StmtKind::Assign(_, expr) => find_lambdas_in_expr(&expr.kind, mutable_vars, captured),
        StmtKind::Return(Some(expr)) => find_lambdas_in_expr(&expr.kind, mutable_vars, captured),
        StmtKind::ReturnError(expr) => find_lambdas_in_expr(&expr.kind, mutable_vars, captured),
        StmtKind::Expr(expr) => find_lambdas_in_expr(&expr.kind, mutable_vars, captured),
        _ => {}
    }
}

fn find_lambdas_in_expr(
    expr: &ExprKind,
    mutable_vars: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    match expr {
        ExprKind::Lambda(params, _, body) => {
            // This is a lambda. Find which mutable outer variables it references.
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            collect_idents_in_block(body, mutable_vars, &param_names, captured);
            // Also recurse into the lambda body for nested lambdas
            for stmt in &body.stmts {
                find_lambdas_in_stmt(&stmt.kind, mutable_vars, captured);
            }
        }
        // Recurse into all sub-expressions
        ExprKind::BinOp(l, _, r) | ExprKind::Range(l, r) | ExprKind::Index(l, r) => {
            find_lambdas_in_expr(&l.kind, mutable_vars, captured);
            find_lambdas_in_expr(&r.kind, mutable_vars, captured);
        }
        ExprKind::UnaryOp(_, e)
        | ExprKind::FieldAccess(e, _)
        | ExprKind::TypeAccess(e, _)
        | ExprKind::ErrorLit(e)
        | ExprKind::As(e, _)
        | ExprKind::AsSafe(e, _)
        | ExprKind::Is(e, _)
        | ExprKind::Try(e) => {
            find_lambdas_in_expr(&e.kind, mutable_vars, captured);
        }
        ExprKind::Call(f, args) => {
            find_lambdas_in_expr(&f.kind, mutable_vars, captured);
            for a in args {
                find_lambdas_in_expr(&a.kind, mutable_vars, captured);
            }
        }
        ExprKind::MethodCall(obj, _, args) => {
            find_lambdas_in_expr(&obj.kind, mutable_vars, captured);
            for a in args {
                find_lambdas_in_expr(&a.kind, mutable_vars, captured);
            }
        }
        ExprKind::StaticMethodCall(_, _, args) => {
            for a in args {
                find_lambdas_in_expr(&a.kind, mutable_vars, captured);
            }
        }
        ExprKind::ArrayLit(elems) | ExprKind::TupleLit(elems) => {
            for e in elems {
                find_lambdas_in_expr(&e.kind, mutable_vars, captured);
            }
        }
        ExprKind::MapLit(entries) => {
            for (k, v) in entries {
                find_lambdas_in_expr(&k.kind, mutable_vars, captured);
                find_lambdas_in_expr(&v.kind, mutable_vars, captured);
            }
        }
        ExprKind::StructLit(_, _, fields, spread) => {
            for (_, e) in fields {
                find_lambdas_in_expr(&e.kind, mutable_vars, captured);
            }
            if let Some(s) = spread {
                find_lambdas_in_expr(&s.kind, mutable_vars, captured);
            }
        }
        ExprKind::EnumVariant(_, _, args) | ExprKind::QualifiedEnumVariant(_, _, _, args) => {
            for a in args {
                find_lambdas_in_expr(&a.kind, mutable_vars, captured);
            }
        }
        ExprKind::If(c, then_block, else_expr) => {
            find_lambdas_in_expr(&c.kind, mutable_vars, captured);
            for s in &then_block.stmts {
                find_lambdas_in_stmt(&s.kind, mutable_vars, captured);
            }
            if let Some(e) = else_expr {
                find_lambdas_in_expr(&e.kind, mutable_vars, captured);
            }
        }
        ExprKind::Match(scrutinee, arms) => {
            find_lambdas_in_expr(&scrutinee.kind, mutable_vars, captured);
            for arm in arms {
                find_lambdas_in_expr(&arm.body.kind, mutable_vars, captured);
            }
        }
        ExprKind::For(_, _, iter, body) | ExprKind::While(iter, body) => {
            find_lambdas_in_expr(&iter.kind, mutable_vars, captured);
            for s in &body.stmts {
                find_lambdas_in_stmt(&s.kind, mutable_vars, captured);
            }
        }
        ExprKind::Loop(body) | ExprKind::Block(body) => {
            for s in &body.stmts {
                find_lambdas_in_stmt(&s.kind, mutable_vars, captured);
            }
        }
        ExprKind::Guard(_, e, body) => {
            find_lambdas_in_expr(&e.kind, mutable_vars, captured);
            for s in &body.stmts {
                find_lambdas_in_stmt(&s.kind, mutable_vars, captured);
            }
        }
        ExprKind::InterpolatedString(parts) => {
            for part in parts {
                if let StringPart::Expr(e) = part {
                    find_lambdas_in_expr(&e.kind, mutable_vars, captured);
                }
            }
        }
        _ => {} // literals, identifiers, nil, etc.
    }
}

/// Walk a block and collect identifiers that are mutable outer variables.
/// Used to find which outer mut vars a lambda body references.
fn collect_idents_in_block(
    block: &Block,
    mutable_vars: &HashSet<String>,
    param_names: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        collect_idents_in_stmt(&stmt.kind, mutable_vars, param_names, captured);
    }
}

fn collect_idents_in_stmt(
    stmt: &StmtKind,
    mutable_vars: &HashSet<String>,
    params: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    match stmt {
        StmtKind::Let(d) => collect_idents_in_expr(&d.value.kind, mutable_vars, params, captured),
        StmtKind::LetTuple(_, e) => collect_idents_in_expr(&e.kind, mutable_vars, params, captured),
        StmtKind::Assign(target, e) => {
            // Check the assignment target for captured mutable variables
            if let AssignTarget::Variable(name) = target {
                if mutable_vars.contains(name) && !params.contains(name) {
                    captured.insert(name.clone());
                }
            }
            collect_idents_in_expr(&e.kind, mutable_vars, params, captured);
        }
        StmtKind::Return(Some(e)) => {
            collect_idents_in_expr(&e.kind, mutable_vars, params, captured)
        }
        StmtKind::ReturnError(e) => collect_idents_in_expr(&e.kind, mutable_vars, params, captured),
        StmtKind::Expr(e) => collect_idents_in_expr(&e.kind, mutable_vars, params, captured),
        _ => {}
    }
}

fn collect_idents_in_expr(
    expr: &ExprKind,
    mutable_vars: &HashSet<String>,
    params: &HashSet<String>,
    captured: &mut HashSet<String>,
) {
    match expr {
        ExprKind::Ident(name) => {
            if mutable_vars.contains(name) && !params.contains(name) {
                captured.insert(name.clone());
            }
        }
        // Recurse into sub-expressions (same structure as find_lambdas_in_expr)
        ExprKind::BinOp(l, _, r) | ExprKind::Range(l, r) | ExprKind::Index(l, r) => {
            collect_idents_in_expr(&l.kind, mutable_vars, params, captured);
            collect_idents_in_expr(&r.kind, mutable_vars, params, captured);
        }
        ExprKind::UnaryOp(_, e)
        | ExprKind::FieldAccess(e, _)
        | ExprKind::TypeAccess(e, _)
        | ExprKind::ErrorLit(e)
        | ExprKind::As(e, _)
        | ExprKind::AsSafe(e, _)
        | ExprKind::Is(e, _)
        | ExprKind::Try(e) => {
            collect_idents_in_expr(&e.kind, mutable_vars, params, captured);
        }
        ExprKind::Call(f, args) => {
            collect_idents_in_expr(&f.kind, mutable_vars, params, captured);
            for a in args {
                collect_idents_in_expr(&a.kind, mutable_vars, params, captured);
            }
        }
        ExprKind::MethodCall(obj, _, args) => {
            collect_idents_in_expr(&obj.kind, mutable_vars, params, captured);
            for a in args {
                collect_idents_in_expr(&a.kind, mutable_vars, params, captured);
            }
        }
        ExprKind::If(c, then_b, else_e) => {
            collect_idents_in_expr(&c.kind, mutable_vars, params, captured);
            for s in &then_b.stmts {
                collect_idents_in_stmt(&s.kind, mutable_vars, params, captured);
            }
            if let Some(e) = else_e {
                collect_idents_in_expr(&e.kind, mutable_vars, params, captured);
            }
        }
        ExprKind::Block(b) | ExprKind::Loop(b) => {
            for s in &b.stmts {
                collect_idents_in_stmt(&s.kind, mutable_vars, params, captured);
            }
        }
        ExprKind::Lambda(_, _, _) => {} // don't recurse into nested lambdas
        _ => {}
    }
}
