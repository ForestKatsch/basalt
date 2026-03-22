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
    Is(Box<TypedExpr>, TypedIsTarget),
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
pub enum TypedIsTarget {
    Type(crate::ast::TypeExpr),
    EnumVariant(String, String),
    QualifiedVariant(String, String, String),
    Expr(Box<TypedExpr>),
}

#[derive(Debug, Clone)]
pub struct TypedMatchArm {
    pub pattern: Pattern,
    pub body: TypedExpr,
    pub bindings: Vec<(String, Type)>,
}

mod checker;
pub use checker::check;
