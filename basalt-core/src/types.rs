/// Basalt Type Checker - Validates types and annotates the AST.
use crate::ast::*;
use std::collections::HashMap;

/// Internal type representation after type checking.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F64,
    Bool,
    String,
    Nil,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Optional(Box<Type>),            // T?
    Result(Box<Type>, Box<Type>),   // T!E
    Function(Vec<Type>, Box<Type>), // params -> return
    Struct(String),                 // named struct type
    Enum(String),                   // named enum type
    Union(Vec<Type>),               // A | B | C
    Error(Box<Type>),               // Error(E) wrapper
    Range,                          // range type for for..in
    // Special types
    Capability(String), // Stdout, Stdin, etc.
    Void,               // for statements that don't produce values
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
        )
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || matches!(self, Type::F64)
    }

    pub fn display_name(&self) -> String {
        match self {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Nil => "nil".to_string(),
            Type::Array(t) => format!("[{}]", t.display_name()),
            Type::Map(k, v) => format!("[{}: {}]", k.display_name(), v.display_name()),
            Type::Tuple(ts) => format!(
                "({})",
                ts.iter()
                    .map(|t| t.display_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Optional(t) => format!("{}?", t.display_name()),
            Type::Result(t, e) => format!("{}!{}", t.display_name(), e.display_name()),
            Type::Function(params, ret) => format!(
                "fn({}) -> {}",
                params
                    .iter()
                    .map(|t| t.display_name())
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.display_name()
            ),
            Type::Struct(name) | Type::Enum(name) => name.clone(),
            Type::Union(members) => members
                .iter()
                .map(|t| t.display_name())
                .collect::<Vec<_>>()
                .join(" | "),
            Type::Error(t) => format!("Error({})", t.display_name()),
            Type::Range => "Range".to_string(),
            Type::Capability(name) => name.clone(),
            Type::Void => "void".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypedProgram {
    pub items: Vec<TypedItem>,
    pub type_info: TypeInfo,
}

#[derive(Debug, Clone, Default)]
pub struct TypeInfo {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub aliases: HashMap<String, Type>,
    pub functions: HashMap<String, FuncInfo>,
    pub modules: HashMap<String, ModuleInfo>,
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub aliases: HashMap<String, Type>,
    pub functions: HashMap<String, FuncInfo>,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub methods: HashMap<String, FuncInfo>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<VariantInfo>,
    pub methods: HashMap<String, FuncInfo>,
}

#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct FuncInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub enum TypedItem {
    Function(TypedFnDef),
    TypeDef(TypeDef),
    Let(TypedLetDecl),
    Import(ImportDecl),
}

#[derive(Debug, Clone)]
pub struct TypedFnDef {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub body: TypedBlock,
}

#[derive(Debug, Clone)]
pub struct TypedBlock {
    pub stmts: Vec<TypedStmt>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum TypedStmt {
    Let(TypedLetDecl),
    Assign(Box<TypedAssignTarget>, Box<TypedExpr>),
    Return(Option<TypedExpr>),
    ReturnError(TypedExpr),
    Break,
    Continue,
    Expr(TypedExpr),
}

#[derive(Debug, Clone)]
pub struct TypedLetDecl {
    pub name: String,
    pub mutable: bool,
    pub ty: Type,
    pub value: TypedExpr,
}

#[derive(Debug, Clone)]
pub enum TypedAssignTarget {
    Variable(String, Type),
    Field(TypedExpr, String, Type),
    Index(TypedExpr, TypedExpr, Type),
}

#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum TypedExprKind {
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),
    InterpolatedString(Vec<TypedStringPart>),
    Nil,
    Ident(String),

    BinOp(Box<TypedExpr>, BinOp, Box<TypedExpr>),
    UnaryOp(UnaryOp, Box<TypedExpr>),

    FieldAccess(Box<TypedExpr>, String),
    Index(Box<TypedExpr>, Box<TypedExpr>),

    Call(Box<TypedExpr>, Vec<TypedExpr>),
    MethodCall(Box<TypedExpr>, String, Vec<TypedExpr>),
    StaticMethodCall(String, String, Vec<TypedExpr>), // Type.method or module.func

    ArrayLit(Vec<TypedExpr>),
    MapLit(Vec<(TypedExpr, TypedExpr)>),
    TupleLit(Vec<TypedExpr>),
    StructLit(String, Vec<(String, TypedExpr)>),
    EnumVariant(String, String, Vec<TypedExpr>),

    ErrorLit(Box<TypedExpr>),

    If(Box<TypedExpr>, TypedBlock, Option<Box<TypedExpr>>),
    Match(Box<TypedExpr>, Vec<TypedMatchArm>),
    For(String, Option<String>, Box<TypedExpr>, TypedBlock),
    While(Box<TypedExpr>, TypedBlock),
    Loop(TypedBlock),
    Guard(Option<String>, Box<TypedExpr>, TypedBlock),
    Block(TypedBlock),

    Lambda(Vec<(String, Type)>, Type, TypedBlock),

    As(Box<TypedExpr>, Type),
    AsSafe(Box<TypedExpr>, Type),
    Is(Box<TypedExpr>, IsTarget),
    Try(Box<TypedExpr>),
    Range(Box<TypedExpr>, Box<TypedExpr>),

    Panic(Vec<TypedExpr>),
}

#[derive(Debug, Clone)]
pub enum TypedStringPart {
    Literal(String),
    Expr(TypedExpr),
}

#[derive(Debug, Clone)]
pub struct TypedMatchArm {
    pub pattern: Pattern,
    pub body: TypedExpr,
    pub bindings: Vec<(String, Type)>,
}

struct TypeChecker {
    type_info: TypeInfo,
    scopes: Vec<HashMap<String, (Type, bool)>>, // name -> (type, mutable)
    current_fn_return: Option<Type>,
    in_loop: bool,
    current_type_name: Option<String>,
}

pub fn check(program: &Program) -> Result<TypedProgram, String> {
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
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_var(&mut self, name: &str, ty: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (ty, mutable));
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

    fn resolve_type_expr(&self, ty: &TypeExpr) -> Result<Type, String> {
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
                Err(format!("unknown type {}.{}", module, name))
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
                Ok(Type::Optional(Box::new(self.resolve_type_expr(inner)?)))
            }
            TypeExpr::Result(ok, err) => Ok(Type::Result(
                Box::new(self.resolve_type_expr(ok)?),
                Box::new(self.resolve_type_expr(err)?),
            )),
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
                        Err("Self used outside of type definition".to_string())
                    }
                } else {
                    Err("Self used outside of type definition".to_string())
                }
            }
        }
    }

    fn resolve_type_name(&self, name: &str) -> Result<Type, String> {
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
            "FileReader" => Ok(Type::Capability("FileReader".to_string())),
            "FileSystem" => Ok(Type::Capability("FileSystem".to_string())),
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
                Err(format!("unknown type '{}'", name))
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
            // Empty array is compatible with any array type
            (Type::Array(inner_from), Type::Array(inner_to)) => {
                self.is_assignable(inner_from, inner_to)
            }
            // Empty map is compatible with any map type
            (Type::Map(kf, vf), Type::Map(kt, vt)) => {
                self.is_assignable(kf, kt) && self.is_assignable(vf, vt)
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

    fn check_program(&mut self, program: &Program) -> Result<TypedProgram, String> {
        // First pass: register all type definitions and function signatures
        self.register_types(program)?;
        self.register_module_types(program)?;
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
            self.define_var(&name, func_type, false);
        }

        // Second pass: type check function bodies
        let mut typed_items = Vec::new();
        for item in &program.items {
            let typed = self.check_item(item)?;
            typed_items.push(typed);
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
                        let typed_fn = self.check_method_def(method, &info)?;
                        self.current_type_name = None;
                        typed_items.push(TypedItem::Function(typed_fn));
                    }
                }
            }
        }

        Ok(TypedProgram {
            items: typed_items,
            type_info: self.type_info.clone(),
        })
    }

    fn register_types(&mut self, program: &Program) -> Result<(), String> {
        // Phase 1: Register type names and fields (no methods yet)
        for item in &program.items {
            if let Item::TypeDef(td) = item {
                self.register_type_def_skeleton(td)?;
            }
        }
        // Phase 2: Register methods (can now reference all types)
        for item in &program.items {
            if let Item::TypeDef(td) = item {
                self.register_type_def_methods(td)?;
            }
        }
        Ok(())
    }

    fn register_module_types(&mut self, program: &Program) -> Result<(), String> {
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
                                .map(|f| Ok((f.name.clone(), self.resolve_type_expr(&f.ty)?)))
                                .collect::<Result<_, String>>()?;
                            let mut methods = HashMap::new();
                            for m in &sdef.methods {
                                let params: Vec<(String, Type)> = m
                                    .params
                                    .iter()
                                    .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                                    .collect::<Result<_, String>>()?;
                                let ret = m
                                    .return_type
                                    .as_ref()
                                    .map(|t| self.resolve_type_expr(t))
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
                                    let fields: Result<Vec<Type>, String> = v
                                        .fields
                                        .iter()
                                        .map(|t| self.resolve_type_expr(t))
                                        .collect();
                                    Ok(VariantInfo {
                                        name: v.name.clone(),
                                        fields: fields?,
                                    })
                                })
                                .collect::<Result<_, String>>()?;
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
                            let resolved = self.resolve_type_expr(ty)?;
                            mod_info.aliases.insert(td.name.clone(), resolved);
                        }
                    },
                    Item::Function(fdef) => {
                        let params: Vec<(String, Type)> = fdef
                            .params
                            .iter()
                            .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                            .collect::<Result<_, String>>()?;
                        let ret = fdef
                            .return_type
                            .as_ref()
                            .map(|t| self.resolve_type_expr(t))
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

            self.type_info.modules.insert(module_name.clone(), mod_info);
        }
        Ok(())
    }

    /// Phase 1: Register type name, fields, and variants (no methods).
    fn register_type_def_skeleton(&mut self, td: &TypeDef) -> Result<(), String> {
        match &td.kind {
            TypeDefKind::Struct(sdef) => {
                self.current_type_name = Some(td.name.clone());
                // Pre-register with empty fields so recursive types can resolve
                self.type_info.structs.insert(
                    td.name.clone(),
                    StructInfo {
                        name: td.name.clone(),
                        fields: Vec::new(),
                        methods: HashMap::new(),
                        parent: td.parent.clone(),
                    },
                );
                // Now resolve field types (can reference Self / own type name)
                let fields: Vec<(String, Type)> = sdef
                    .fields
                    .iter()
                    .map(|f| Ok((f.name.clone(), self.resolve_type_expr(&f.ty)?)))
                    .collect::<Result<_, String>>()?;
                self.type_info.structs.get_mut(&td.name).unwrap().fields = fields;
                self.current_type_name = None;
            }
            TypeDefKind::Enum(edef) => {
                self.current_type_name = Some(td.name.clone());
                // Pre-register with empty variants so recursive types can resolve
                self.type_info.enums.insert(
                    td.name.clone(),
                    EnumInfo {
                        name: td.name.clone(),
                        variants: Vec::new(),
                        methods: HashMap::new(),
                    },
                );
                // Now resolve variant field types (can reference own type)
                let variants: Vec<VariantInfo> = edef
                    .variants
                    .iter()
                    .map(|v| {
                        let fields: Result<Vec<Type>, String> =
                            v.fields.iter().map(|t| self.resolve_type_expr(t)).collect();
                        Ok(VariantInfo {
                            name: v.name.clone(),
                            fields: fields?,
                        })
                    })
                    .collect::<Result<_, String>>()?;
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
    fn register_type_def_methods(&mut self, td: &TypeDef) -> Result<(), String> {
        self.current_type_name = Some(td.name.clone());
        match &td.kind {
            TypeDefKind::Struct(sdef) => {
                let mut methods = HashMap::new();
                for m in &sdef.methods {
                    let params: Vec<(String, Type)> = m
                        .params
                        .iter()
                        .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                        .collect::<Result<_, String>>()?;
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
                        .collect::<Result<_, String>>()?;
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

    fn register_functions(&mut self, program: &Program) -> Result<(), String> {
        for item in &program.items {
            if let Item::Function(fdef) = item {
                let params: Vec<(String, Type)> = fdef
                    .params
                    .iter()
                    .map(|p| Ok((p.name.clone(), self.resolve_type_expr(&p.ty)?)))
                    .collect::<Result<_, String>>()?;
                let ret = fdef
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type_expr(t))
                    .transpose()?
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

    fn check_item(&mut self, item: &Item) -> Result<TypedItem, String> {
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

    fn check_fn_def(&mut self, fdef: &FnDef) -> Result<TypedFnDef, String> {
        let info = self
            .type_info
            .functions
            .get(&fdef.name)
            .cloned()
            .ok_or_else(|| format!("unknown function '{}'", fdef.name))?;

        self.push_scope();
        let old_return = self.current_fn_return.take();
        self.current_fn_return = Some(info.return_type.clone());

        // Bind parameters
        for (name, ty) in &info.params {
            self.define_var(name, ty.clone(), false);
        }

        let body = self.check_block(&fdef.body)?;

        self.current_fn_return = old_return;
        self.pop_scope();

        Ok(TypedFnDef {
            name: fdef.name.clone(),
            params: info.params,
            return_type: info.return_type,
            body,
        })
    }

    fn check_method_def(&mut self, method: &FnDef, info: &FuncInfo) -> Result<TypedFnDef, String> {
        self.push_scope();
        let old_return = self.current_fn_return.take();
        self.current_fn_return = Some(info.return_type.clone());

        // Bind parameters
        for (name, ty) in &info.params {
            self.define_var(name, ty.clone(), false);
        }

        let body = self.check_block(&method.body)?;

        self.current_fn_return = old_return;
        self.pop_scope();

        Ok(TypedFnDef {
            name: method.name.clone(),
            params: info.params.clone(),
            return_type: info.return_type.clone(),
            body,
        })
    }

    fn check_let_decl(&mut self, decl: &LetDecl) -> Result<TypedLetDecl, String> {
        let value = self.check_expr(&decl.value)?;

        let ty = if let Some(ref type_expr) = decl.ty {
            let declared = self.resolve_type_expr(type_expr)?;
            // Special case: integer literal (or negated literal) assigned to narrow integer type
            let int_literal_value = Self::extract_int_literal(&value);
            let is_int_literal_narrowing =
                int_literal_value.is_some() && value.ty.is_integer() && declared.is_integer();
            if !is_int_literal_narrowing && !self.is_assignable(&value.ty, &declared) {
                return Err(format!(
                    "type mismatch in let '{}': expected {}, got {}",
                    decl.name,
                    declared.display_name(),
                    value.ty.display_name()
                ));
            }
            // Compile-time range check for integer literals assigned to narrow types
            if let Some(n) = int_literal_value {
                Self::check_int_literal_range(n, &declared, &decl.name)?;
            }
            declared
        } else {
            value.ty.clone()
        };

        self.define_var(&decl.name, ty.clone(), decl.mutable);

        Ok(TypedLetDecl {
            name: decl.name.clone(),
            mutable: decl.mutable,
            ty,
            value,
        })
    }

    fn check_block(&mut self, block: &Block) -> Result<TypedBlock, String> {
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

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<TypedStmt, String> {
        match stmt {
            Stmt::Let(decl) => {
                let typed = self.check_let_decl(decl)?;
                Ok(TypedStmt::Let(typed))
            }
            Stmt::Assign(target, value) => {
                let typed_value = self.check_expr(value)?;
                let typed_target = match target {
                    AssignTarget::Variable(name) => {
                        let (var_ty, mutable) = self
                            .lookup_var(name)
                            .ok_or_else(|| format!("undefined variable '{}'", name))?;
                        if !mutable {
                            return Err(format!("cannot assign to immutable variable '{}'", name));
                        }
                        if !self.is_assignable(&typed_value.ty, &var_ty) {
                            return Err(format!(
                                "type mismatch in assignment to '{}': expected {}, got {}",
                                name,
                                var_ty.display_name(),
                                typed_value.ty.display_name()
                            ));
                        }
                        TypedAssignTarget::Variable(name.clone(), var_ty)
                    }
                    AssignTarget::Field(obj, field) => {
                        let typed_obj = self.check_expr(obj)?;
                        let field_ty = self.get_field_type(&typed_obj.ty, field)?;
                        if !self.is_assignable(&typed_value.ty, &field_ty) {
                            return Err(format!(
                                "type mismatch in field assignment: expected {}, got {}",
                                field_ty.display_name(),
                                typed_value.ty.display_name()
                            ));
                        }
                        TypedAssignTarget::Field(typed_obj, field.clone(), field_ty)
                    }
                    AssignTarget::Index(obj, idx) => {
                        let typed_obj = self.check_expr(obj)?;
                        let typed_idx = self.check_expr(idx)?;
                        let elem_ty = self.get_index_type(&typed_obj.ty, &typed_idx.ty)?;
                        if !self.is_assignable(&typed_value.ty, &elem_ty) {
                            return Err(format!(
                                "type mismatch in index assignment: expected {}, got {}",
                                elem_ty.display_name(),
                                typed_value.ty.display_name()
                            ));
                        }
                        TypedAssignTarget::Index(typed_obj, typed_idx, elem_ty)
                    }
                };
                Ok(TypedStmt::Assign(Box::new(typed_target), Box::new(typed_value)))
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    let typed = self.check_expr(expr)?;
                    if let Some(ref ret_ty) = self.current_fn_return {
                        if !self.is_assignable(&typed.ty, ret_ty) {
                            return Err(format!(
                                "return type mismatch: expected {}, got {}",
                                ret_ty.display_name(),
                                typed.ty.display_name()
                            ));
                        }
                    }
                    Ok(TypedStmt::Return(Some(typed)))
                } else {
                    Ok(TypedStmt::Return(None))
                }
            }
            Stmt::ReturnError(expr) => {
                let typed = self.check_expr(expr)?;
                // Check that enclosing function returns a result type
                if let Some(Type::Result(_, err_ty)) = &self.current_fn_return {
                    if !self.is_assignable(&typed.ty, err_ty) {
                        return Err(format!(
                            "error type mismatch: expected {}, got {}",
                            err_ty.display_name(),
                            typed.ty.display_name()
                        ));
                    }
                }
                Ok(TypedStmt::ReturnError(typed))
            }
            Stmt::Break => {
                if !self.in_loop {
                    return Err("break outside of loop".to_string());
                }
                Ok(TypedStmt::Break)
            }
            Stmt::Continue => {
                if !self.in_loop {
                    return Err("continue outside of loop".to_string());
                }
                Ok(TypedStmt::Continue)
            }
            Stmt::Expr(expr) => {
                let typed = self.check_expr(expr)?;
                Ok(TypedStmt::Expr(typed))
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<TypedExpr, String> {
        match expr {
            Expr::IntLit(n) => Ok(TypedExpr {
                kind: TypedExprKind::IntLit(*n),
                ty: Type::I64,
            }),
            Expr::FloatLit(f) => Ok(TypedExpr {
                kind: TypedExprKind::FloatLit(*f),
                ty: Type::F64,
            }),
            Expr::BoolLit(b) => Ok(TypedExpr {
                kind: TypedExprKind::BoolLit(*b),
                ty: Type::Bool,
            }),
            Expr::StringLit(s) => Ok(TypedExpr {
                kind: TypedExprKind::StringLit(s.clone()),
                ty: Type::String,
            }),
            Expr::InterpolatedString(parts) => {
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
                })
            }
            Expr::Nil => Ok(TypedExpr {
                kind: TypedExprKind::Nil,
                ty: Type::Nil,
            }),
            Expr::Ident(name) => {
                if name == "panic" {
                    // panic is a global builtin
                    Ok(TypedExpr {
                        kind: TypedExprKind::Ident("panic".to_string()),
                        ty: Type::Function(vec![Type::String], Box::new(Type::Nil)),
                    })
                } else if let Some((ty, _)) = self.lookup_var(name) {
                    Ok(TypedExpr {
                        kind: TypedExprKind::Ident(name.clone()),
                        ty,
                    })
                } else if let Some(func_info) = self.type_info.functions.get(name) {
                    let func_type = Type::Function(
                        func_info.params.iter().map(|(_, t)| t.clone()).collect(),
                        Box::new(func_info.return_type.clone()),
                    );
                    Ok(TypedExpr {
                        kind: TypedExprKind::Ident(name.clone()),
                        ty: func_type,
                    })
                } else {
                    // Could be module name
                    if self.type_info.modules.contains_key(name) {
                        Ok(TypedExpr {
                            kind: TypedExprKind::Ident(name.clone()),
                            ty: Type::Nil, // module is not a value
                        })
                    } else {
                        Err(format!("undefined variable '{}'", name))
                    }
                }
            }
            Expr::TypeIdent(name) => {
                // Type as value (for static access like Type.method)
                Ok(TypedExpr {
                    kind: TypedExprKind::Ident(name.clone()),
                    ty: Type::Nil,
                })
            }
            Expr::BinOp(left, op, right) => {
                let typed_left = self.check_expr(left)?;
                let typed_right = self.check_expr(right)?;
                let result_ty = self.check_binop(&typed_left.ty, op, &typed_right.ty)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::BinOp(
                        Box::new(typed_left),
                        op.clone(),
                        Box::new(typed_right),
                    ),
                    ty: result_ty,
                })
            }
            Expr::UnaryOp(op, expr) => {
                let typed = self.check_expr(expr)?;
                let result_ty = match op {
                    UnaryOp::Neg => {
                        if !typed.ty.is_numeric() {
                            return Err(format!("cannot negate {}", typed.ty.display_name()));
                        }
                        typed.ty.clone()
                    }
                    UnaryOp::Not => {
                        if typed.ty != Type::Bool {
                            return Err(format!(
                                "logical NOT requires bool, got {}",
                                typed.ty.display_name()
                            ));
                        }
                        Type::Bool
                    }
                };
                Ok(TypedExpr {
                    kind: TypedExprKind::UnaryOp(op.clone(), Box::new(typed)),
                    ty: result_ty,
                })
            }
            Expr::FieldAccess(obj, field) => {
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
                            });
                        }
                        return Err(format!(
                            "tuple index {} out of bounds (length {})",
                            idx,
                            types.len()
                        ));
                    }
                }

                let field_ty = self.get_field_type(&typed_obj.ty, field)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::FieldAccess(Box::new(typed_obj), field.clone()),
                    ty: field_ty,
                })
            }
            Expr::TypeAccess(obj, name) => {
                // module.Type access or Type.StaticField
                let typed_obj = self.check_expr(obj)?;
                // The type access itself just passes through
                Ok(TypedExpr {
                    kind: TypedExprKind::FieldAccess(Box::new(typed_obj), name.clone()),
                    ty: Type::Nil,
                })
            }
            Expr::Index(obj, idx) => {
                let typed_obj = self.check_expr(obj)?;
                let typed_idx = self.check_expr(idx)?;
                let elem_ty = self.get_index_type(&typed_obj.ty, &typed_idx.ty)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::Index(Box::new(typed_obj), Box::new(typed_idx)),
                    ty: elem_ty,
                })
            }
            Expr::Call(func, args) => self.check_call(func, args),
            Expr::MethodCall(obj, method, args) => self.check_method_call(obj, method, args),
            Expr::StaticMethodCall(type_or_module, method, args) => {
                self.check_static_method_call(type_or_module, method, args)
            }
            Expr::ArrayLit(elems) => {
                if elems.is_empty() {
                    return Ok(TypedExpr {
                        kind: TypedExprKind::ArrayLit(vec![]),
                        ty: Type::Array(Box::new(Type::Nil)), // inferred from context
                    });
                }
                let mut typed_elems = Vec::new();
                let first = self.check_expr(&elems[0])?;
                let elem_ty = first.ty.clone();
                typed_elems.push(first);
                for elem in &elems[1..] {
                    let typed = self.check_expr(elem)?;
                    if !self.is_assignable(&typed.ty, &elem_ty) {
                        return Err(format!(
                            "array element type mismatch: expected {}, got {}",
                            elem_ty.display_name(),
                            typed.ty.display_name()
                        ));
                    }
                    typed_elems.push(typed);
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::ArrayLit(typed_elems),
                    ty: Type::Array(Box::new(elem_ty)),
                })
            }
            Expr::MapLit(entries) => {
                if entries.is_empty() {
                    return Ok(TypedExpr {
                        kind: TypedExprKind::MapLit(vec![]),
                        ty: Type::Map(Box::new(Type::Nil), Box::new(Type::Nil)),
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
                        return Err(format!(
                            "map key type mismatch: expected {}, got {}",
                            key_ty.display_name(),
                            typed_k.ty.display_name()
                        ));
                    }
                    if !self.is_assignable(&typed_v.ty, &val_ty) {
                        return Err(format!(
                            "map value type mismatch: expected {}, got {}",
                            val_ty.display_name(),
                            typed_v.ty.display_name()
                        ));
                    }
                    typed_entries.push((typed_k, typed_v));
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::MapLit(typed_entries),
                    ty: Type::Map(Box::new(key_ty), Box::new(val_ty)),
                })
            }
            Expr::TupleLit(elems) => {
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
                })
            }
            Expr::StructLit(type_name, module, fields) => {
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

                let info =
                    struct_info.ok_or_else(|| format!("unknown struct type '{}'", full_name))?;

                let mut typed_fields = Vec::new();
                let mut provided = std::collections::HashSet::new();

                for (name, expr) in fields {
                    if !provided.insert(name.clone()) {
                        return Err(format!("duplicate field '{}' in struct literal", name));
                    }
                    let field_ty = info
                        .fields
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| {
                            format!("unknown field '{}' on struct '{}'", name, full_name)
                        })?;
                    let typed = self.check_expr(expr)?;
                    if !self.is_assignable(&typed.ty, &field_ty) {
                        return Err(format!(
                            "field '{}' type mismatch: expected {}, got {}",
                            name,
                            field_ty.display_name(),
                            typed.ty.display_name()
                        ));
                    }
                    typed_fields.push((name.clone(), typed));
                }

                // Check all fields are provided
                for (name, _) in &info.fields {
                    if !provided.contains(name) {
                        return Err(format!(
                            "missing field '{}' in struct literal for '{}'",
                            name, full_name
                        ));
                    }
                }

                Ok(TypedExpr {
                    kind: TypedExprKind::StructLit(full_name.clone(), typed_fields),
                    ty: Type::Struct(full_name),
                })
            }
            Expr::EnumVariant(type_name, variant, args) => {
                self.check_enum_variant(type_name, variant, args, None)
            }
            Expr::QualifiedEnumVariant(module, type_name, variant, args) => {
                self.check_enum_variant(type_name, variant, args, Some(module))
            }
            Expr::ErrorLit(expr) => {
                let typed = self.check_expr(expr)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::ErrorLit(Box::new(typed.clone())),
                    ty: Type::Error(Box::new(typed.ty)),
                })
            }
            Expr::If(cond, then_block, else_expr) => {
                let typed_cond = self.check_expr(cond)?;

                // Check for type narrowing: if x is T
                let narrowing = if let Expr::Is(inner, IsTarget::Type(ty)) = cond.as_ref() {
                    if let Expr::Ident(name) = inner.as_ref() {
                        Some((name.clone(), self.resolve_type_expr(ty)?))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if typed_cond.ty != Type::Bool {
                    return Err(format!(
                        "if condition must be bool, got {}",
                        typed_cond.ty.display_name()
                    ));
                }

                // Apply narrowing in then block
                if let Some((name, narrow_ty)) = &narrowing {
                    self.push_scope();
                    self.define_var(name, narrow_ty.clone(), false);
                }

                let typed_then = self.check_block(then_block)?;

                if narrowing.is_some() {
                    self.pop_scope();
                }

                let typed_else = if let Some(else_expr) = else_expr {
                    Some(Box::new(self.check_expr(else_expr)?))
                } else {
                    None
                };

                let ty = if let Some(ref else_typed) = typed_else {
                    // Both branches must be compatible
                    if self.is_assignable(&typed_then.ty, &else_typed.ty) {
                        else_typed.ty.clone()
                    } else if self.is_assignable(&else_typed.ty, &typed_then.ty) {
                        typed_then.ty.clone()
                    } else {
                        Type::Nil
                    }
                } else {
                    Type::Nil
                };

                Ok(TypedExpr {
                    kind: TypedExprKind::If(Box::new(typed_cond), typed_then, typed_else),
                    ty,
                })
            }
            Expr::Match(scrutinee, arms) => {
                let typed_scrutinee = self.check_expr(scrutinee)?;
                let mut typed_arms = Vec::new();
                let mut result_ty: Option<Type> = None;

                for arm in arms {
                    let (bindings, pattern_checked) =
                        self.check_pattern(&arm.pattern, &typed_scrutinee.ty)?;

                    self.push_scope();
                    for (name, ty) in &bindings {
                        self.define_var(name, ty.clone(), false);
                    }

                    let typed_body = self.check_expr(&arm.body)?;
                    self.pop_scope();

                    if let Some(ref rty) = result_ty {
                        // Types should be compatible
                        if !self.is_assignable(&typed_body.ty, rty)
                            && !self.is_assignable(rty, &typed_body.ty)
                        {
                            // Allow if both are nil (returns in all arms)
                            if typed_body.ty != Type::Nil && *rty != Type::Nil {
                                // Try to create a union
                            }
                        }
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
                self.check_match_exhaustiveness(&typed_scrutinee.ty, arms)?;

                Ok(TypedExpr {
                    kind: TypedExprKind::Match(Box::new(typed_scrutinee), typed_arms),
                    ty: result_ty.unwrap_or(Type::Nil),
                })
            }
            Expr::For(var1, var2, iterable, body) => {
                let typed_iterable = self.check_expr(iterable)?;
                self.push_scope();
                let old_in_loop = self.in_loop;
                self.in_loop = true;

                match &typed_iterable.ty {
                    Type::Array(elem_ty) => {
                        self.define_var(var1, *elem_ty.clone(), false);
                        if let Some(var2) = var2 {
                            self.define_var(var2, Type::I64, false);
                        }
                    }
                    Type::Map(key_ty, val_ty) => {
                        self.define_var(var1, *key_ty.clone(), false);
                        if let Some(var2) = var2 {
                            self.define_var(var2, *val_ty.clone(), false);
                        }
                    }
                    Type::String => {
                        self.define_var(var1, Type::String, false);
                        if let Some(var2) = var2 {
                            self.define_var(var2, Type::I64, false);
                        }
                    }
                    Type::Range => {
                        self.define_var(var1, Type::I64, false);
                    }
                    _ => {
                        return Err(format!(
                            "cannot iterate over {}",
                            typed_iterable.ty.display_name()
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
                })
            }
            Expr::While(cond, body) => {
                let typed_cond = self.check_expr(cond)?;
                if typed_cond.ty != Type::Bool {
                    return Err(format!(
                        "while condition must be bool, got {}",
                        typed_cond.ty.display_name()
                    ));
                }
                let old_in_loop = self.in_loop;
                self.in_loop = true;
                let typed_body = self.check_block(body)?;
                self.in_loop = old_in_loop;
                Ok(TypedExpr {
                    kind: TypedExprKind::While(Box::new(typed_cond), typed_body),
                    ty: Type::Nil,
                })
            }
            Expr::Loop(body) => {
                let old_in_loop = self.in_loop;
                self.in_loop = true;
                let typed_body = self.check_block(body)?;
                self.in_loop = old_in_loop;
                Ok(TypedExpr {
                    kind: TypedExprKind::Loop(typed_body),
                    ty: Type::Nil,
                })
            }
            Expr::Guard(binding, expr, else_block) => {
                if let Some(name) = binding {
                    // guard let name = expr else { ... }
                    let typed_expr = self.check_expr(expr)?;
                    let typed_else = self.check_block(else_block)?;

                    // The else block MUST diverge (return, break, continue, panic)
                    if !Self::block_diverges(&typed_else) {
                        return Err(
                            "guard else block must diverge (return, break, continue, or panic)"
                                .to_string(),
                        );
                    }

                    // Determine unwrapped type
                    let unwrapped_ty = match &typed_expr.ty {
                        Type::Optional(inner) => *inner.clone(),
                        Type::Result(ok, _) => *ok.clone(),
                        _ => typed_expr.ty.clone(),
                    };

                    // The binding is available in the enclosing scope (after the guard)
                    self.define_var(name, unwrapped_ty.clone(), false);

                    Ok(TypedExpr {
                        kind: TypedExprKind::Guard(
                            Some(name.clone()),
                            Box::new(typed_expr),
                            typed_else,
                        ),
                        ty: Type::Nil,
                    })
                } else {
                    // guard condition else { ... }
                    let typed_cond = self.check_expr(expr)?;
                    if typed_cond.ty != Type::Bool {
                        return Err(format!(
                            "guard condition must be bool, got {}",
                            typed_cond.ty.display_name()
                        ));
                    }
                    let typed_else = self.check_block(else_block)?;

                    // The else block MUST diverge
                    if !Self::block_diverges(&typed_else) {
                        return Err(
                            "guard else block must diverge (return, break, continue, or panic)"
                                .to_string(),
                        );
                    }

                    Ok(TypedExpr {
                        kind: TypedExprKind::Guard(None, Box::new(typed_cond), typed_else),
                        ty: Type::Nil,
                    })
                }
            }
            Expr::Block(block) => {
                let typed = self.check_block(block)?;
                let ty = typed.ty.clone();
                Ok(TypedExpr {
                    kind: TypedExprKind::Block(typed),
                    ty,
                })
            }
            Expr::Lambda(params, ret_type, body) => {
                let mut param_types = Vec::new();
                self.push_scope();
                for p in params {
                    let ty = self.resolve_type_expr(&p.ty)?;
                    self.define_var(&p.name, ty.clone(), false);
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

                let func_type = Type::Function(
                    param_types.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(ret_ty.clone()),
                );
                Ok(TypedExpr {
                    kind: TypedExprKind::Lambda(param_types, ret_ty, typed_body),
                    ty: func_type,
                })
            }
            Expr::As(expr, ty) => {
                let typed = self.check_expr(expr)?;
                let target = self.resolve_type_expr(ty)?;
                // Validate conversion pair
                self.check_conversion(&typed.ty, &target, false)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::As(Box::new(typed), target.clone()),
                    ty: target,
                })
            }
            Expr::AsSafe(expr, ty) => {
                let typed = self.check_expr(expr)?;
                let target = self.resolve_type_expr(ty)?;
                self.check_conversion(&typed.ty, &target, true)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::AsSafe(Box::new(typed), target.clone()),
                    ty: Type::Optional(Box::new(target)),
                })
            }
            Expr::Is(expr, target) => {
                let typed = self.check_expr(expr)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::Is(Box::new(typed), target.clone()),
                    ty: Type::Bool,
                })
            }
            Expr::Try(expr) => {
                let typed = self.check_expr(expr)?;
                match &typed.ty {
                    Type::Result(ok, _) => {
                        let ok_ty = *ok.clone();
                        Ok(TypedExpr {
                            kind: TypedExprKind::Try(Box::new(typed)),
                            ty: ok_ty,
                        })
                    }
                    _ => Err(format!(
                        "? operator requires result type, got {}",
                        typed.ty.display_name()
                    )),
                }
            }
            Expr::Range(start, end) => {
                let typed_start = self.check_expr(start)?;
                let typed_end = self.check_expr(end)?;
                if !typed_start.ty.is_integer() {
                    return Err(format!(
                        "range start must be integer, got {}",
                        typed_start.ty.display_name()
                    ));
                }
                if !typed_end.ty.is_integer() {
                    return Err(format!(
                        "range end must be integer, got {}",
                        typed_end.ty.display_name()
                    ));
                }
                Ok(TypedExpr {
                    kind: TypedExprKind::Range(Box::new(typed_start), Box::new(typed_end)),
                    ty: Type::Range,
                })
            }
        }
    }

    fn check_call(&mut self, func: &Expr, args: &[Expr]) -> Result<TypedExpr, String> {
        // Handle panic() specially
        if let Expr::Ident(name) = func {
            if name == "panic" {
                let mut typed_args = Vec::new();
                for arg in args {
                    typed_args.push(self.check_expr(arg)?);
                }
                return Ok(TypedExpr {
                    kind: TypedExprKind::Panic(typed_args),
                    ty: Type::Nil, // panic never returns, but for type purposes
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
                    return Err(format!(
                        "function expects {} arguments, got {}",
                        param_types.len(),
                        typed_args.len()
                    ));
                }
                for (i, (param_ty, arg)) in param_types.iter().zip(typed_args.iter()).enumerate() {
                    if !self.is_assignable(&arg.ty, param_ty) {
                        return Err(format!(
                            "argument {} type mismatch: expected {}, got {}",
                            i + 1,
                            param_ty.display_name(),
                            arg.ty.display_name()
                        ));
                    }
                }
                let result_ty = *ret_type.clone();
                Ok(TypedExpr {
                    kind: TypedExprKind::Call(Box::new(typed_func), typed_args),
                    ty: result_ty,
                })
            }
            _ => Err(format!(
                "cannot call value of type {}",
                func_ty.display_name()
            )),
        }
    }

    fn check_method_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<TypedExpr, String> {
        // If obj is a TypeIdent, this is a static method call: Type.method(args)
        if let Expr::TypeIdent(type_name) = obj {
            return self.check_static_method_call(type_name, method, args);
        }

        let typed_obj = self.check_expr(obj)?;
        let mut typed_args = Vec::new();
        for arg in args {
            typed_args.push(self.check_expr(arg)?);
        }

        // Check built-in methods
        let result_ty = self.check_builtin_method(&typed_obj.ty, method, &typed_args)?;

        Ok(TypedExpr {
            kind: TypedExprKind::MethodCall(Box::new(typed_obj), method.to_string(), typed_args),
            ty: result_ty,
        })
    }

    fn check_builtin_method(
        &self,
        obj_ty: &Type,
        method: &str,
        args: &[TypedExpr],
    ) -> Result<Type, String> {
        match obj_ty {
            Type::String => self.check_string_method(method, args),
            Type::Array(elem_ty) => self.check_array_method(elem_ty, method, args),
            Type::Map(key_ty, val_ty) => self.check_map_method(key_ty, val_ty, method, args),
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
                            return Err(format!(
                                "method '{}' expects {} arguments, got {}",
                                method,
                                expected_args.len(),
                                args.len()
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
                    if let Some(info) = mod_info.structs.get(name.split('.').next_back().unwrap_or(name))
                    {
                        if let Some(method_info) = info.methods.get(method) {
                            return Ok(method_info.return_type.clone());
                        }
                        if method == "clone" && args.is_empty() {
                            return Ok(obj_ty.clone());
                        }
                    }
                }
                Err(format!(
                    "unknown method '{}' on {}",
                    method,
                    obj_ty.display_name()
                ))
            }
            Type::Enum(name) => {
                if let Some(info) = self.type_info.enums.get(name) {
                    if let Some(method_info) = info.methods.get(method) {
                        return Ok(method_info.return_type.clone());
                    }
                }
                Err(format!(
                    "unknown method '{}' on {}",
                    method,
                    obj_ty.display_name()
                ))
            }
            Type::Capability(cap_name) => self.check_capability_method(cap_name, method, args),
            _ => Err(format!(
                "unknown method '{}' on {}",
                method,
                obj_ty.display_name()
            )),
        }
    }

    fn check_string_method(&self, method: &str, args: &[TypedExpr]) -> Result<Type, String> {
        match method {
            "split" => {
                if args.len() != 1 {
                    return Err("split takes 1 argument".to_string());
                }
                Ok(Type::Array(Box::new(Type::String)))
            }
            "trim" | "trim_start" | "trim_end" | "upper" | "lower" => {
                if !args.is_empty() {
                    return Err(format!("{} takes 0 arguments", method));
                }
                Ok(Type::String)
            }
            "replace" => {
                if args.len() != 2 {
                    return Err("replace takes 2 arguments".to_string());
                }
                Ok(Type::String)
            }
            "find" => {
                if args.len() != 1 {
                    return Err("find takes 1 argument".to_string());
                }
                Ok(Type::Optional(Box::new(Type::I64)))
            }
            "substring" => {
                if args.len() != 2 {
                    return Err("substring takes 2 arguments".to_string());
                }
                Ok(Type::String)
            }
            "starts_with" | "ends_with" | "contains" => {
                if args.len() != 1 {
                    return Err(format!("{} takes 1 argument", method));
                }
                Ok(Type::Bool)
            }
            "repeat" => {
                if args.len() != 1 {
                    return Err("repeat takes 1 argument".to_string());
                }
                Ok(Type::String)
            }
            "char_at" => {
                if args.len() != 1 {
                    return Err("char_at takes 1 argument".to_string());
                }
                Ok(Type::String)
            }
            _ => Err(format!("unknown string method '{}'", method)),
        }
    }

    fn check_array_method(
        &self,
        elem_ty: &Type,
        method: &str,
        args: &[TypedExpr],
    ) -> Result<Type, String> {
        match method {
            "push" => {
                if args.len() != 1 {
                    return Err("push takes 1 argument".to_string());
                }
                Ok(Type::Nil)
            }
            "pop" => {
                if !args.is_empty() {
                    return Err("pop takes 0 arguments".to_string());
                }
                Ok(elem_ty.clone())
            }
            "insert" => {
                if args.len() != 2 {
                    return Err("insert takes 2 arguments".to_string());
                }
                Ok(Type::Nil)
            }
            "remove" => {
                if args.len() != 1 {
                    return Err("remove takes 1 argument".to_string());
                }
                Ok(Type::Nil)
            }
            "sort" | "reverse" => {
                if !args.is_empty() {
                    return Err(format!("{} takes 0 arguments", method));
                }
                Ok(Type::Nil)
            }
            "join" => {
                if args.len() != 1 {
                    return Err("join takes 1 argument".to_string());
                }
                Ok(Type::String)
            }
            "contains" => {
                if args.len() != 1 {
                    return Err("contains takes 1 argument".to_string());
                }
                Ok(Type::Bool)
            }
            "clone" => {
                if !args.is_empty() {
                    return Err("clone takes 0 arguments".to_string());
                }
                Ok(Type::Array(Box::new(elem_ty.clone())))
            }
            _ => Err(format!("unknown array method '{}'", method)),
        }
    }

    fn check_map_method(
        &self,
        key_ty: &Type,
        val_ty: &Type,
        method: &str,
        args: &[TypedExpr],
    ) -> Result<Type, String> {
        match method {
            "get" => {
                if args.len() != 1 {
                    return Err("get takes 1 argument".to_string());
                }
                Ok(Type::Optional(Box::new(val_ty.clone())))
            }
            "contains_key" => {
                if args.len() != 1 {
                    return Err("contains_key takes 1 argument".to_string());
                }
                Ok(Type::Bool)
            }
            "keys" => {
                if !args.is_empty() {
                    return Err("keys takes 0 arguments".to_string());
                }
                Ok(Type::Array(Box::new(key_ty.clone())))
            }
            "values" => {
                if !args.is_empty() {
                    return Err("values takes 0 arguments".to_string());
                }
                Ok(Type::Array(Box::new(val_ty.clone())))
            }
            "remove" => {
                if args.len() != 1 {
                    return Err("remove takes 1 argument".to_string());
                }
                Ok(Type::Nil)
            }
            "clone" => {
                if !args.is_empty() {
                    return Err("clone takes 0 arguments".to_string());
                }
                Ok(Type::Map(
                    Box::new(key_ty.clone()),
                    Box::new(val_ty.clone()),
                ))
            }
            _ => Err(format!("unknown map method '{}'", method)),
        }
    }

    fn check_capability_method(
        &self,
        cap: &str,
        method: &str,
        args: &[TypedExpr],
    ) -> Result<Type, String> {
        match cap {
            "Stdout" => match method {
                "println" | "print" => {
                    if args.len() != 1 {
                        return Err(format!("{} takes 1 argument", method));
                    }
                    Ok(Type::Nil)
                }
                "flush" => {
                    if !args.is_empty() {
                        return Err("flush takes 0 arguments".to_string());
                    }
                    Ok(Type::Nil)
                }
                _ => Err(format!("unknown method '{}' on Stdout", method)),
            },
            "Stdin" => match method {
                "read_line" => {
                    if !args.is_empty() {
                        return Err("read_line takes 0 arguments".to_string());
                    }
                    Ok(Type::String)
                }
                "read_key" => {
                    if !args.is_empty() {
                        return Err("read_key takes 0 arguments".to_string());
                    }
                    Ok(Type::String)
                }
                _ => Err(format!("unknown method '{}' on Stdin", method)),
            },
            _ => Err(format!("unknown capability '{}'", cap)),
        }
    }

    fn check_static_method_call(
        &mut self,
        name: &str,
        method: &str,
        args: &[Expr],
    ) -> Result<TypedExpr, String> {
        let mut typed_args = Vec::new();
        for arg in args {
            typed_args.push(self.check_expr(arg)?);
        }

        // Check if name is a module
        if let Some(mod_info) = self.type_info.modules.get(name).cloned() {
            // module.function(args)
            if let Some(func_info) = mod_info.functions.get(method) {
                if func_info.params.len() != typed_args.len() {
                    return Err(format!(
                        "function '{}.{}' expects {} arguments, got {}",
                        name,
                        method,
                        func_info.params.len(),
                        typed_args.len()
                    ));
                }
                return Ok(TypedExpr {
                    kind: TypedExprKind::StaticMethodCall(
                        name.to_string(),
                        method.to_string(),
                        typed_args,
                    ),
                    ty: func_info.return_type.clone(),
                });
            }
            // Could be module.Type(args) - enum variant with single field
            if let Some(_enum_info) = mod_info.enums.get(method) {
                // This is actually accessing a type, not calling a function
                return Err(format!("'{}' is a type, not a function", method));
            }
        }

        // Check if name is a type with a static method
        if let Some(struct_info) = self.type_info.structs.get(name).cloned() {
            if let Some(method_info) = struct_info.methods.get(method) {
                if !method_info.is_method {
                    // Static method
                    if method_info.params.len() != typed_args.len() {
                        return Err(format!(
                            "static method '{}.{}' expects {} arguments, got {}",
                            name,
                            method,
                            method_info.params.len(),
                            typed_args.len()
                        ));
                    }
                    return Ok(TypedExpr {
                        kind: TypedExprKind::StaticMethodCall(
                            name.to_string(),
                            method.to_string(),
                            typed_args,
                        ),
                        ty: method_info.return_type.clone(),
                    });
                }
            }
        }

        // Check for enum variant construction that looks like a static method call
        if let Some(enum_info) = self.type_info.enums.get(name).cloned() {
            if let Some(variant) = enum_info.variants.iter().find(|v| v.name == method) {
                if variant.fields.len() != typed_args.len() {
                    return Err(format!(
                        "enum variant '{}.{}' expects {} arguments, got {}",
                        name,
                        method,
                        variant.fields.len(),
                        typed_args.len()
                    ));
                }
                return Ok(TypedExpr {
                    kind: TypedExprKind::EnumVariant(
                        name.to_string(),
                        method.to_string(),
                        typed_args,
                    ),
                    ty: Type::Enum(name.to_string()),
                });
            }
        }

        Err(format!("unknown function or method '{}.{}'", name, method))
    }

    fn check_enum_variant(
        &mut self,
        type_name: &str,
        variant: &str,
        args: &[Expr],
        module: Option<&String>,
    ) -> Result<TypedExpr, String> {
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

        let info = enum_info.ok_or_else(|| format!("unknown enum type '{}'", full_name))?;

        let variant_info = info
            .variants
            .iter()
            .find(|v| v.name == variant)
            .ok_or_else(|| format!("unknown variant '{}.{}'", full_name, variant))?;

        if variant_info.fields.len() != args.len() {
            return Err(format!(
                "variant '{}.{}' expects {} arguments, got {}",
                full_name,
                variant,
                variant_info.fields.len(),
                args.len()
            ));
        }

        let mut typed_args = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let typed = self.check_expr(arg)?;
            if !self.is_assignable(&typed.ty, &variant_info.fields[i]) {
                return Err(format!(
                    "variant field type mismatch: expected {}, got {}",
                    variant_info.fields[i].display_name(),
                    typed.ty.display_name()
                ));
            }
            typed_args.push(typed);
        }

        Ok(TypedExpr {
            kind: TypedExprKind::EnumVariant(full_name.clone(), variant.to_string(), typed_args),
            ty: Type::Enum(full_name),
        })
    }

    fn check_binop(&self, left: &Type, op: &BinOp, right: &Type) -> Result<Type, String> {
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
                Err(format!(
                    "cannot add {} and {}",
                    left.display_name(),
                    right.display_name()
                ))
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if left == right && left.is_numeric() {
                    return Ok(left.clone());
                }
                Err(format!(
                    "cannot apply {:?} to {} and {}",
                    op,
                    left.display_name(),
                    right.display_name()
                ))
            }
            BinOp::Pow => {
                if left == right && left.is_numeric() {
                    return Ok(left.clone());
                }
                Err(format!(
                    "cannot apply ** to {} and {}",
                    left.display_name(),
                    right.display_name()
                ))
            }
            BinOp::Eq | BinOp::NotEq => {
                if left == right || (left == &Type::Nil || right == &Type::Nil) {
                    Ok(Type::Bool)
                } else {
                    Err(format!(
                        "cannot compare {} and {} for equality",
                        left.display_name(),
                        right.display_name()
                    ))
                }
            }
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if left == right && (left.is_numeric() || *left == Type::String) {
                    Ok(Type::Bool)
                } else {
                    Err(format!(
                        "cannot compare {} and {}",
                        left.display_name(),
                        right.display_name()
                    ))
                }
            }
            BinOp::And | BinOp::Or => {
                if *left == Type::Bool && *right == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(format!(
                        "logical operators require bool operands, got {} and {}",
                        left.display_name(),
                        right.display_name()
                    ))
                }
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::ShiftLeft | BinOp::ShiftRight => {
                if left == right && left.is_integer() {
                    Ok(left.clone())
                } else {
                    Err(format!(
                        "bitwise operators require matching integer types, got {} and {}",
                        left.display_name(),
                        right.display_name()
                    ))
                }
            }
        }
    }

    fn check_conversion(&self, from: &Type, to: &Type, _is_safe: bool) -> Result<(), String> {
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

        Err(format!(
            "cannot convert {} to {}",
            from.display_name(),
            to.display_name()
        ))
    }

    fn get_field_type(&self, ty: &Type, field: &str) -> Result<Type, String> {
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
                Err(format!("unknown field '{}' on {}", field, name))
            }
            _ => Err(format!(
                "cannot access field '{}' on {}",
                field,
                ty.display_name()
            )),
        }
    }

    fn get_index_type(&self, obj_ty: &Type, idx_ty: &Type) -> Result<Type, String> {
        match obj_ty {
            Type::Array(elem_ty) => {
                if !idx_ty.is_integer() {
                    return Err(format!(
                        "array index must be integer, got {}",
                        idx_ty.display_name()
                    ));
                }
                Ok(*elem_ty.clone())
            }
            Type::Map(key_ty, val_ty) => {
                if !self.is_assignable(idx_ty, key_ty) {
                    return Err(format!(
                        "map key type mismatch: expected {}, got {}",
                        key_ty.display_name(),
                        idx_ty.display_name()
                    ));
                }
                Ok(*val_ty.clone())
            }
            _ => Err(format!("cannot index into {}", obj_ty.display_name())),
        }
    }

    /// Extract an integer literal value, including through unary negation.
    fn extract_int_literal(expr: &TypedExpr) -> Option<i64> {
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
    fn check_int_literal_range(n: i64, ty: &Type, name: &str) -> Result<(), String> {
        let (min, max): (i64, i64) = match ty {
            Type::I8 => (i8::MIN as i64, i8::MAX as i64),
            Type::I16 => (i16::MIN as i64, i16::MAX as i64),
            Type::I32 => (i32::MIN as i64, i32::MAX as i64),
            Type::U8 => (0, u8::MAX as i64),
            Type::U16 => (0, u16::MAX as i64),
            Type::U32 => (0, u32::MAX as i64),
            _ => return Ok(()), // i64, u64, or non-integer — no check needed
        };
        if n < min || n > max {
            return Err(format!(
                "integer literal {} out of range for {} in '{}'",
                n,
                ty.display_name(),
                name
            ));
        }
        Ok(())
    }

    /// Check whether a block always diverges (return, break, continue, panic).
    fn block_diverges(block: &TypedBlock) -> bool {
        if let Some(last) = block.stmts.last() {
            match last {
                TypedStmt::Return(_) | TypedStmt::ReturnError(_) | TypedStmt::Break
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
    ) -> Result<(), String> {
        // Check if any arm is a catch-all (wildcard or binding)
        let has_catch_all = arms.iter().any(|arm| {
            matches!(
                arm.pattern,
                Pattern::Wildcard | Pattern::Binding(_)
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
                    match &arm.pattern {
                        Pattern::EnumVariant(_, variant, _)
                        | Pattern::QualifiedEnumVariant(_, _, variant, _) => {
                            if let Some(idx) = enum_info
                                .variants
                                .iter()
                                .position(|v| v.name == *variant)
                            {
                                covered[idx] = true;
                            }
                        }
                        Pattern::IsEnumVariant(_, variant) => {
                            if let Some(idx) = enum_info
                                .variants
                                .iter()
                                .position(|v| v.name == *variant)
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
                    return Err(format!(
                        "non-exhaustive match on '{}': missing variant(s) {}",
                        name,
                        missing.join(", ")
                    ));
                }
            }
            Type::Bool => {
                let mut has_true = false;
                let mut has_false = false;
                for arm in arms {
                    match &arm.pattern {
                        Pattern::BoolLit(true) => has_true = true,
                        Pattern::BoolLit(false) => has_false = true,
                        _ => {}
                    }
                }
                if !has_true || !has_false {
                    return Err(
                        "non-exhaustive match on bool: missing true or false branch".to_string(),
                    );
                }
            }
            // For integers, strings, etc. we can't check exhaustiveness
            // without a wildcard, but we don't error — they may use guard/return patterns
            _ => {}
        }
        Ok(())
    }

    fn check_pattern(
        &self,
        pattern: &Pattern,
        scrutinee_ty: &Type,
    ) -> Result<(Vec<(String, Type)>, Pattern), String> {
        let mut bindings = Vec::new();

        match pattern {
            Pattern::Wildcard => {}
            Pattern::IntLit(_)
            | Pattern::FloatLit(_)
            | Pattern::BoolLit(_)
            | Pattern::StringLit(_)
            | Pattern::Nil => {}
            Pattern::Binding(name) => {
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
            Pattern::EnumVariant(type_name, variant, bound_names) => {
                if let Some(info) = self.type_info.enums.get(type_name) {
                    if let Some(v) = info.variants.iter().find(|v| v.name == *variant) {
                        if bound_names.len() != v.fields.len() {
                            return Err(format!(
                                "pattern for '{}.{}' expects {} bindings, got {}",
                                type_name,
                                variant,
                                v.fields.len(),
                                bound_names.len()
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
            Pattern::QualifiedEnumVariant(module, type_name, variant, bound_names) => {
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
            Pattern::Error(name) => {
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
            Pattern::IsType(ty) => {
                // Type narrowing in match
                if let Ok(_resolved) = self.resolve_type_expr(ty) {
                    // No new binding, but the scrutinee gets narrowed
                }
            }
            Pattern::IsEnumVariant(_, _) => {}
        }

        Ok((bindings, pattern.clone()))
    }
}
