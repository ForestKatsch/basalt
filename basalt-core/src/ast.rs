/// Basalt AST - Abstract Syntax Tree definitions.
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
    pub modules: HashMap<String, Vec<Item>>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(FnDef),
    TypeDef(TypeDef),
    Let(LetDecl),
    Import(ImportDecl),
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(LetDecl),
    Assign(AssignTarget, Expr),
    Return(Option<Expr>),
    ReturnError(Expr),
    Break,
    Continue,
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct LetDecl {
    pub name: String,
    pub mutable: bool,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Variable(String),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),
    InterpolatedString(Vec<StringPart>),
    Nil,
    Ident(String),
    TypeIdent(String),

    // Binary and unary
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),

    // Access
    FieldAccess(Box<Expr>, String),
    TypeAccess(Box<Expr>, String), // e.g., Color.Red - access on type
    Index(Box<Expr>, Box<Expr>),

    // Call
    Call(Box<Expr>, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    StaticMethodCall(String, String, Vec<Expr>), // Type.method(args)

    // Collections
    ArrayLit(Vec<Expr>),
    MapLit(Vec<(Expr, Expr)>),
    TupleLit(Vec<Expr>),

    // Struct construction
    StructLit(String, Option<String>, Vec<(String, Expr)>), // TypeName, optional module, fields

    // Enum variant construction
    EnumVariant(String, String, Vec<Expr>), // TypeName, Variant, args
    QualifiedEnumVariant(String, String, String, Vec<Expr>), // module, TypeName, Variant, args

    // Error construction: !(expr)
    ErrorLit(Box<Expr>),

    // Control flow
    If(Box<Expr>, Block, Option<Box<Expr>>), // condition, then, else
    Match(Box<Expr>, Vec<MatchArm>),
    For(String, Option<String>, Box<Expr>, Block), // var, optional second var, iterable, body
    While(Box<Expr>, Block),
    Loop(Block),
    Guard(Option<String>, Box<Expr>, Block), // optional let binding, condition/expr, else block
    Block(Block),

    // Lambda
    Lambda(Vec<Param>, Option<TypeExpr>, Block),

    // Type operations
    As(Box<Expr>, TypeExpr),     // expr as Type
    AsSafe(Box<Expr>, TypeExpr), // expr as? Type
    Is(Box<Expr>, IsTarget),     // expr is ...
    Try(Box<Expr>),              // expr?

    // Range
    Range(Box<Expr>, Box<Expr>), // start..end
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum IsTarget {
    Type(TypeExpr),                           // is SomeType
    EnumVariant(String, String),              // is Type.Variant
    QualifiedVariant(String, String, String), // is module.Type.Variant
    Expr(Box<Expr>),                          // is someExpr (reference identity)
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),
    Nil,
    Binding(String),
    EnumVariant(String, String, Vec<String>), // Type, Variant, bindings
    QualifiedEnumVariant(String, String, String, Vec<String>), // module, Type, Variant, bindings
    Error(String),                            // !name
    IsType(TypeExpr),                         // is T
    IsEnumVariant(String, String),            // is Type.Variant
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(String),                          // i64, string, bool, MyType, etc.
    Qualified(String, String),              // module.Type
    Array(Box<TypeExpr>),                   // [T]
    Map(Box<TypeExpr>, Box<TypeExpr>),      // [K: V]
    Tuple(Vec<TypeExpr>),                   // (T1, T2, ...)
    Optional(Box<TypeExpr>),                // T?
    Result(Box<TypeExpr>, Box<TypeExpr>),   // T!E
    Function(Vec<TypeExpr>, Box<TypeExpr>), // fn(A, B) -> R
    Union(Vec<TypeExpr>),                   // A | B | C
    SelfType,                               // Self
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub parent: Option<String>,
    pub kind: TypeDefKind,
}

#[derive(Debug, Clone)]
pub enum TypeDefKind {
    Struct(StructDef),
    Enum(EnumDef),
    Alias(TypeExpr),
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub fields: Vec<FieldDef>,
    pub methods: Vec<FnDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub variants: Vec<VariantDef>,
    pub methods: Vec<FnDef>,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<TypeExpr>, // empty for unit variants
}
