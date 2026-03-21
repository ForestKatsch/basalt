/// Basalt Parser - Converts tokens into AST.
use crate::ast::*;
use crate::lexer::{SpannedToken, StringPart as LexStringPart, Token};

pub fn parse(tokens: Vec<SpannedToken>) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    lines: Vec<usize>,
    cols: Vec<usize>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        let lines: Vec<usize> = tokens.iter().map(|st| st.line).collect();
        let cols: Vec<usize> = tokens.iter().map(|st| st.col).collect();
        Parser {
            tokens: tokens.into_iter().map(|st| st.token).collect(),
            lines,
            cols,
            pos: 0,
        }
    }

    fn current_span(&self) -> (usize, usize) {
        let idx = self.pos.min(self.lines.len().saturating_sub(1));
        (
            self.lines.get(idx).copied().unwrap_or(0),
            self.cols.get(idx).copied().unwrap_or(0),
        )
    }

    fn error(&self, msg: impl std::fmt::Display) -> String {
        let (line, col) = self.current_span();
        format!("[line {}:{}] {}", line, col, msg)
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek_ahead(&self, n: usize) -> &Token {
        self.tokens.get(self.pos + n).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.advance();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) {
            Ok(())
        } else {
            Err(self.error(format!("expected {:?}, got {:?}", expected, tok)))
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Ident(name) => Ok(name),
            tok => Err(self.error(format!("expected identifier, got {:?}", tok))),
        }
    }

    fn expect_type_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::TypeIdent(name) => Ok(name),
            tok => Err(self.error(format!("expected type identifier, got {:?}", tok))),
        }
    }

    fn skip_newlines(&mut self) {
        while *self.peek() == Token::Newline {
            self.advance();
        }
    }

    fn expect_newline_or_eof(&mut self) -> Result<(), String> {
        match self.peek() {
            Token::Newline => {
                self.advance();
                Ok(())
            }
            Token::Eof => Ok(()),
            Token::RBrace => Ok(()), // Allow statements before closing brace
            tok => Err(self.error(format!("expected newline, got {:?}", tok))),
        }
    }

    fn parse_program(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        self.skip_newlines();

        while *self.peek() != Token::Eof {
            let item = self.parse_item()?;
            items.push(item);
            self.skip_newlines();
        }

        Ok(Program {
            items,
            modules: std::collections::HashMap::new(),
        })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        match self.peek().clone() {
            Token::Fn => {
                let func = self.parse_fn_def()?;
                Ok(Item::Function(func))
            }
            Token::Type => {
                let td = self.parse_type_def()?;
                Ok(Item::TypeDef(td))
            }
            Token::Let => {
                let decl = self.parse_let_decl()?;
                self.expect_newline_or_eof()?;
                Ok(Item::Let(decl))
            }
            Token::Import => {
                let imp = self.parse_import()?;
                self.expect_newline_or_eof()?;
                Ok(Item::Import(imp))
            }
            tok => Err(self.error(format!(
                "expected item (fn, type, let, import), got {:?}",
                tok
            ))),
        }
    }

    fn parse_import(&mut self) -> Result<ImportDecl, String> {
        self.expect(&Token::Import)?;
        let path = match self.advance() {
            Token::StringLit(s) => s,
            tok => return Err(self.error(format!("expected string after import, got {:?}", tok))),
        };
        let alias = if *self.peek() == Token::As {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        Ok(ImportDecl { path, alias })
    }

    fn parse_fn_def(&mut self) -> Result<FnDef, String> {
        self.expect(&Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        let return_type = if *self.peek() == Token::Arrow {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.skip_newlines();
        let body = self.parse_block()?;

        Ok(FnDef {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        if *self.peek() == Token::RParen {
            return Ok(params);
        }

        loop {
            self.skip_newlines();
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let ty = self.parse_type_expr()?;
            params.push(Param { name, ty });

            if *self.peek() == Token::Comma {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        Ok(params)
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, String> {
        let base = self.parse_base_type()?;

        // Check for union: T | U | V
        if *self.peek() == Token::Pipe {
            let mut members = vec![base];
            while *self.peek() == Token::Pipe {
                self.advance();
                members.push(self.parse_base_type()?);
            }
            return Ok(TypeExpr::Union(members));
        }

        Ok(base)
    }

    fn parse_base_type(&mut self) -> Result<TypeExpr, String> {
        let mut ty = match self.peek().clone() {
            Token::Ident(ref name) => {
                let name = name.clone();
                match name.as_str() {
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f64"
                    | "bool" | "string" | "nil" => {
                        self.advance();
                        TypeExpr::Named(name)
                    }
                    _ => {
                        // Could be module.Type
                        self.advance();
                        if *self.peek() == Token::Dot {
                            // Look ahead to see if next is TypeIdent
                            if let Token::TypeIdent(_) = self.peek_ahead(1) {
                                self.advance(); // consume dot
                                let type_name = self.expect_type_ident()?;
                                TypeExpr::Qualified(name, type_name)
                            } else {
                                TypeExpr::Named(name)
                            }
                        } else {
                            TypeExpr::Named(name)
                        }
                    }
                }
            }
            Token::TypeIdent(ref name) => {
                let name = name.clone();
                self.advance();
                if name == "Self" {
                    TypeExpr::SelfType
                } else {
                    TypeExpr::Named(name)
                }
            }
            Token::LBracket => {
                self.advance();
                if *self.peek() == Token::RBracket {
                    return Err(self.error("empty array type needs element type: [T]"));
                }
                let inner = self.parse_type_expr()?;
                if *self.peek() == Token::Colon {
                    // Map type [K: V]
                    self.advance();
                    let value = self.parse_type_expr()?;
                    self.expect(&Token::RBracket)?;
                    TypeExpr::Map(Box::new(inner), Box::new(value))
                } else {
                    self.expect(&Token::RBracket)?;
                    TypeExpr::Array(Box::new(inner))
                }
            }
            Token::LParen => {
                self.advance();
                let first = self.parse_type_expr()?;
                if *self.peek() == Token::Comma {
                    let mut types = vec![first];
                    while *self.peek() == Token::Comma {
                        self.advance();
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        types.push(self.parse_type_expr()?);
                    }
                    self.expect(&Token::RParen)?;
                    TypeExpr::Tuple(types)
                } else {
                    self.expect(&Token::RParen)?;
                    // Single-element parens is just grouping
                    first
                }
            }
            Token::Fn => {
                self.advance();
                self.expect(&Token::LParen)?;
                let mut param_types = Vec::new();
                if *self.peek() != Token::RParen {
                    loop {
                        param_types.push(self.parse_type_expr()?);
                        if *self.peek() == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(&Token::RParen)?;
                let ret = if *self.peek() == Token::Arrow {
                    self.advance();
                    self.parse_type_expr()?
                } else {
                    TypeExpr::Named("nil".to_string())
                };
                TypeExpr::Function(param_types, Box::new(ret))
            }
            tok => return Err(self.error(format!("expected type, got {:?}", tok))),
        };

        // Check for postfix type operators: ?, !
        loop {
            match self.peek() {
                Token::Question => {
                    self.advance();
                    ty = TypeExpr::Optional(Box::new(ty));
                }
                Token::Bang => {
                    self.advance();
                    let err_ty = self.parse_base_type()?;
                    ty = TypeExpr::Result(Box::new(ty), Box::new(err_ty));
                }
                _ => break,
            }
        }

        Ok(ty)
    }

    fn parse_type_def(&mut self) -> Result<TypeDef, String> {
        self.expect(&Token::Type)?;
        let name = self.expect_type_ident()?;

        // Check for parent type: type Foo: Bar { ... }
        let parent = if *self.peek() == Token::Colon {
            self.advance();
            Some(self.expect_type_ident()?)
        } else {
            None
        };

        // Check for type alias: type Foo = ...
        if *self.peek() == Token::Eq {
            self.advance();
            let ty = self.parse_type_expr()?;
            self.expect_newline_or_eof()?;
            return Ok(TypeDef {
                name,
                parent,
                kind: TypeDefKind::Alias(ty),
            });
        }

        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        // Parse type members: fields, methods, variants
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut variants = Vec::new();

        while *self.peek() != Token::RBrace {
            self.skip_newlines();
            if *self.peek() == Token::RBrace {
                break;
            }

            match self.peek().clone() {
                Token::Fn => {
                    let method = self.parse_fn_def()?;
                    methods.push(method);
                    self.skip_newlines();
                }
                Token::TypeIdent(variant_name) => {
                    // Enum variant
                    self.advance();
                    let variant_fields = if *self.peek() == Token::LParen {
                        self.advance();
                        let mut vf = Vec::new();
                        if *self.peek() != Token::RParen {
                            loop {
                                vf.push(self.parse_type_expr()?);
                                if *self.peek() == Token::Comma {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(&Token::RParen)?;
                        vf
                    } else {
                        Vec::new()
                    };
                    // Optional comma after variant
                    if *self.peek() == Token::Comma {
                        self.advance();
                    }
                    variants.push(VariantDef {
                        name: variant_name,
                        fields: variant_fields,
                    });
                    self.skip_newlines();
                }
                Token::Ident(field_name) => {
                    // Field: name: Type
                    self.advance();
                    self.expect(&Token::Colon)?;
                    let ty = self.parse_type_expr()?;
                    fields.push(FieldDef {
                        name: field_name,
                        ty,
                    });
                    // Allow comma or newline after field
                    if *self.peek() == Token::Comma {
                        self.advance();
                    }
                    self.skip_newlines();
                }
                tok => return Err(self.error(format!("unexpected token in type body: {:?}", tok))),
            }
        }

        self.expect(&Token::RBrace)?;

        let kind = if !variants.is_empty() {
            TypeDefKind::Enum(EnumDef { variants, methods })
        } else {
            TypeDefKind::Struct(StructDef { fields, methods })
        };

        Ok(TypeDef { name, parent, kind })
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut stmts = Vec::new();

        while *self.peek() != Token::RBrace {
            if *self.peek() == Token::Eof {
                return Err(self.error("unterminated block"));
            }

            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
            self.skip_newlines();
        }

        self.expect(&Token::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::Let => {
                let decl = self.parse_let_decl()?;
                self.expect_newline_or_eof()?;
                Ok(Stmt::Let(decl))
            }
            Token::Return => {
                self.advance();
                if *self.peek() == Token::Bang && *self.peek_ahead(1) == Token::LParen {
                    // return !(expr)
                    self.advance(); // !
                    self.advance(); // (
                    let expr = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    self.expect_newline_or_eof()?;
                    Ok(Stmt::ReturnError(expr))
                } else if matches!(self.peek(), Token::Newline | Token::RBrace | Token::Eof) {
                    self.expect_newline_or_eof()?;
                    Ok(Stmt::Return(None))
                } else {
                    let expr = self.parse_expr()?;
                    self.expect_newline_or_eof()?;
                    Ok(Stmt::Return(Some(expr)))
                }
            }
            Token::Break => {
                self.advance();
                self.expect_newline_or_eof()?;
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                self.expect_newline_or_eof()?;
                Ok(Stmt::Continue)
            }
            _ => {
                let expr = self.parse_expr()?;
                // Check for assignment
                if *self.peek() == Token::Eq {
                    self.advance();
                    let target = self.expr_to_assign_target(expr)?;
                    let value = self.parse_expr()?;
                    self.expect_newline_or_eof()?;
                    Ok(Stmt::Assign(target, value))
                } else {
                    self.expect_newline_or_eof()?;
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn expr_to_assign_target(&self, expr: Expr) -> Result<AssignTarget, String> {
        match expr {
            Expr::Ident(name) => Ok(AssignTarget::Variable(name)),
            Expr::FieldAccess(obj, field) => Ok(AssignTarget::Field(obj, field)),
            Expr::Index(obj, idx) => Ok(AssignTarget::Index(obj, idx)),
            _ => Err(self.error("invalid assignment target")),
        }
    }

    fn parse_let_decl(&mut self) -> Result<LetDecl, String> {
        self.expect(&Token::Let)?;
        let mutable = if *self.peek() == Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_ident()?;
        let ty = if *self.peek() == Token::Colon {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(LetDecl {
            name,
            mutable,
            ty,
            value,
        })
    }

    // Expression parsing with precedence climbing
    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_range_expr()
    }

    // Precedence 1: Range (..)
    fn parse_range_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_or_expr()?;
        if *self.peek() == Token::DotDot {
            self.advance();
            let right = self.parse_or_expr()?;
            Ok(Expr::Range(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    // Precedence 2: Logical OR (||)
    fn parse_or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and_expr()?;
        while *self.peek() == Token::PipePipe {
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 3: Logical AND (&&)
    fn parse_and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitor_expr()?;
        while *self.peek() == Token::AmpAmp {
            self.advance();
            let right = self.parse_bitor_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 4: Bitwise OR (|)
    fn parse_bitor_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitxor_expr()?;
        while *self.peek() == Token::Pipe {
            self.advance();
            let right = self.parse_bitxor_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::BitOr, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 5: Bitwise XOR (^)
    fn parse_bitxor_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitand_expr()?;
        while *self.peek() == Token::Caret {
            self.advance();
            let right = self.parse_bitand_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::BitXor, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 6: Bitwise AND (&)
    fn parse_bitand_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality_expr()?;
        while *self.peek() == Token::Ampersand {
            self.advance();
            let right = self.parse_equality_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::BitAnd, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 7: Equality (==, !=, is)
    fn parse_equality_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison_expr()?;
        loop {
            match self.peek() {
                Token::EqEq => {
                    self.advance();
                    let right = self.parse_comparison_expr()?;
                    left = Expr::BinOp(Box::new(left), BinOp::Eq, Box::new(right));
                }
                Token::BangEq => {
                    self.advance();
                    let right = self.parse_comparison_expr()?;
                    left = Expr::BinOp(Box::new(left), BinOp::NotEq, Box::new(right));
                }
                Token::Is => {
                    self.advance();
                    let target = self.parse_is_target()?;
                    left = Expr::Is(Box::new(left), target);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_is_target(&mut self) -> Result<IsTarget, String> {
        match self.peek().clone() {
            // Type name or enum variant
            Token::TypeIdent(name) => {
                self.advance();
                if *self.peek() == Token::Dot {
                    // Could be Type.Variant
                    self.advance();
                    let variant = self.expect_type_ident()?;
                    Ok(IsTarget::EnumVariant(name, variant))
                } else {
                    Ok(IsTarget::Type(TypeExpr::Named(name)))
                }
            }
            // Primitive type name
            Token::Ident(ref name) if is_primitive_type(name) => {
                let name = name.clone();
                self.advance();
                // Check for optional/result suffix
                let mut ty = TypeExpr::Named(name);
                if *self.peek() == Token::Question {
                    self.advance();
                    ty = TypeExpr::Optional(Box::new(ty));
                }
                Ok(IsTarget::Type(ty))
            }
            // module.Type.Variant or module.Type
            Token::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                if *self.peek() == Token::Dot {
                    if let Token::TypeIdent(_) = self.peek_ahead(1) {
                        self.advance(); // dot
                        let type_name = self.expect_type_ident()?;
                        if *self.peek() == Token::Dot {
                            // module.Type.Variant
                            self.advance();
                            let variant = self.expect_type_ident()?;
                            Ok(IsTarget::QualifiedVariant(name, type_name, variant))
                        } else {
                            Ok(IsTarget::Type(TypeExpr::Qualified(name, type_name)))
                        }
                    } else {
                        // Just an expression
                        Ok(IsTarget::Expr(Box::new(Expr::Ident(name))))
                    }
                } else {
                    Ok(IsTarget::Expr(Box::new(Expr::Ident(name))))
                }
            }
            Token::Nil => {
                self.advance();
                Ok(IsTarget::Type(TypeExpr::Named("nil".to_string())))
            }
            _ => {
                // Expression identity test
                let expr = self.parse_comparison_expr()?;
                Ok(IsTarget::Expr(Box::new(expr)))
            }
        }
    }

    // Precedence 8: Comparison (<, <=, >, >=)
    fn parse_comparison_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift_expr()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::LtEq => BinOp::LtEq,
                Token::Gt => BinOp::Gt,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift_expr()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 9: Shift (<<, >>)
    fn parse_shift_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive_expr()?;
        loop {
            let op = match self.peek() {
                Token::ShiftLeft => BinOp::ShiftLeft,
                Token::ShiftRight => BinOp::ShiftRight,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive_expr()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 10: Additive (+, -)
    fn parse_additive_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative_expr()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative_expr()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 11: Multiplicative (*, /, %)
    fn parse_multiplicative_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power_expr()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power_expr()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    // Precedence 12: Power (**) - right associative
    fn parse_power_expr(&mut self) -> Result<Expr, String> {
        let base = self.parse_as_expr()?;
        if *self.peek() == Token::StarStar {
            self.advance();
            let exp = self.parse_power_expr()?; // right-associative: recurse
            Ok(Expr::BinOp(Box::new(base), BinOp::Pow, Box::new(exp)))
        } else {
            Ok(base)
        }
    }

    // Precedence 13: as / as?
    fn parse_as_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary_expr()?;
        loop {
            if *self.peek() == Token::As {
                self.advance();
                if *self.peek() == Token::Question {
                    self.advance();
                    let ty = self.parse_base_type()?;
                    expr = Expr::AsSafe(Box::new(expr), ty);
                } else {
                    let ty = self.parse_base_type()?;
                    expr = Expr::As(Box::new(expr), ty);
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    // Precedence 14: Unary (-, !)
    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(expr)))
            }
            Token::Bang => {
                // Check if this is !(expr) - error literal
                if *self.peek_ahead(1) == Token::LParen {
                    self.advance(); // !
                    self.advance(); // (
                    let expr = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    let error = Expr::ErrorLit(Box::new(expr));
                    // Continue parsing postfix on the error value
                    self.parse_postfix(error)
                } else {
                    self.advance();
                    let expr = self.parse_unary_expr()?;
                    Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(expr)))
                }
            }
            _ => self.parse_postfix_expr(),
        }
    }

    // Precedence 15: Postfix (., (), [], ?)
    fn parse_postfix_expr(&mut self) -> Result<Expr, String> {
        let expr = self.parse_primary()?;
        self.parse_postfix(expr)
    }

    fn parse_postfix(&mut self, mut expr: Expr) -> Result<Expr, String> {
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    match self.peek().clone() {
                        Token::Ident(name) => {
                            self.advance();
                            // Check for method call
                            if *self.peek() == Token::LParen {
                                self.advance();
                                let args = self.parse_args()?;
                                self.expect(&Token::RParen)?;
                                expr = Expr::MethodCall(Box::new(expr), name, args);
                            } else {
                                expr = Expr::FieldAccess(Box::new(expr), name);
                            }
                        }
                        Token::TypeIdent(name) => {
                            self.advance();
                            // Enum variant construction: Type.Variant or Type.Variant(args)
                            if *self.peek() == Token::LParen {
                                self.advance();
                                let args = self.parse_args()?;
                                self.expect(&Token::RParen)?;

                                // Determine if this is EnumVariant, StaticMethodCall, or QualifiedEnumVariant
                                match &expr {
                                    Expr::TypeIdent(type_name) => {
                                        expr = Expr::EnumVariant(type_name.clone(), name, args);
                                    }
                                    Expr::Ident(module_name) => {
                                        // module.Type(args) -> could be static method call or
                                        // qualified type access
                                        expr =
                                            Expr::StaticMethodCall(module_name.clone(), name, args);
                                    }
                                    _ => {
                                        expr = Expr::MethodCall(Box::new(expr), name, args);
                                    }
                                }
                            } else {
                                // Type.Variant with no args, or module.Type access
                                match &expr {
                                    Expr::TypeIdent(type_name) => {
                                        expr = Expr::EnumVariant(type_name.clone(), name, vec![]);
                                    }
                                    Expr::Ident(_) => {
                                        expr = Expr::TypeAccess(Box::new(expr), name);
                                    }
                                    _ => {
                                        expr = Expr::TypeAccess(Box::new(expr), name);
                                    }
                                }
                            }
                        }
                        Token::IntLit(idx) => {
                            // Tuple index: t.0, t.1
                            self.advance();
                            expr = Expr::FieldAccess(Box::new(expr), idx.to_string());
                        }
                        tok => return Err(self.error(format!("expected field name after '.', got {:?}", tok))),
                    }
                }
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                Token::Question => {
                    self.advance();
                    expr = Expr::Try(Box::new(expr));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        self.skip_newlines();
        if *self.peek() == Token::RParen {
            return Ok(args);
        }

        loop {
            self.skip_newlines();
            args.push(self.parse_expr()?);
            self.skip_newlines();
            if *self.peek() == Token::Comma {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::FloatLit(f) => {
                self.advance();
                Ok(Expr::FloatLit(f))
            }
            Token::True => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            Token::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Token::InterpolatedString(parts) => {
                self.advance();
                let ast_parts = self.convert_string_parts(parts)?;
                Ok(Expr::InterpolatedString(ast_parts))
            }
            Token::Ident(name) => {
                self.advance();
                // Check if this is a qualified access: module.Type { ... } or module.Type.Variant
                if *self.peek() == Token::Dot {
                    if let Token::TypeIdent(type_name) = self.peek_ahead(1).clone() {
                        // module.Type - check context
                        // Could be: module.Type { fields }, module.Type.Variant, module.Type.method()
                        self.advance(); // consume .
                        self.advance(); // consume TypeIdent

                        // Check what follows
                        match self.peek() {
                            Token::LBrace => {
                                // module.Type { ... } - struct construction
                                self.advance();
                                self.skip_newlines();
                                let fields = self.parse_struct_fields()?;
                                self.expect(&Token::RBrace)?;
                                return Ok(Expr::StructLit(type_name, Some(name), fields));
                            }
                            Token::Dot => {
                                // module.Type.Variant or module.Type.method
                                self.advance();
                                match self.peek().clone() {
                                    Token::TypeIdent(variant) => {
                                        self.advance();
                                        if *self.peek() == Token::LParen {
                                            self.advance();
                                            let args = self.parse_args()?;
                                            self.expect(&Token::RParen)?;
                                            return Ok(Expr::QualifiedEnumVariant(
                                                name, type_name, variant, args,
                                            ));
                                        }
                                        return Ok(Expr::QualifiedEnumVariant(
                                            name,
                                            type_name,
                                            variant,
                                            vec![],
                                        ));
                                    }
                                    Token::Ident(method) => {
                                        self.advance();
                                        if *self.peek() == Token::LParen {
                                            self.advance();
                                            let args = self.parse_args()?;
                                            self.expect(&Token::RParen)?;
                                            let type_expr = Expr::TypeAccess(
                                                Box::new(Expr::Ident(name)),
                                                type_name,
                                            );
                                            return Ok(Expr::MethodCall(
                                                Box::new(type_expr),
                                                method,
                                                args,
                                            ));
                                        }
                                        let type_expr = Expr::TypeAccess(
                                            Box::new(Expr::Ident(name)),
                                            type_name,
                                        );
                                        return Ok(Expr::FieldAccess(Box::new(type_expr), method));
                                    }
                                    _ => {
                                        return Ok(Expr::TypeAccess(
                                            Box::new(Expr::Ident(name)),
                                            type_name,
                                        ));
                                    }
                                }
                            }
                            Token::LParen => {
                                // module.Type(args) - could be static method call or variant
                                self.advance();
                                let args = self.parse_args()?;
                                self.expect(&Token::RParen)?;
                                return Ok(Expr::StaticMethodCall(name, type_name, args));
                            }
                            _ => {
                                return Ok(Expr::TypeAccess(
                                    Box::new(Expr::Ident(name)),
                                    type_name,
                                ));
                            }
                        }
                    }
                }
                Ok(Expr::Ident(name))
            }
            Token::TypeIdent(name) => {
                self.advance();
                // Check for struct construction: Type { ... }
                match self.peek() {
                    Token::LBrace => {
                        // Lookahead to distinguish struct lit from block
                        // If we see ident: then it's a struct lit
                        if self.is_struct_lit_ahead() {
                            self.advance(); // consume {
                            self.skip_newlines();
                            let fields = self.parse_struct_fields()?;
                            self.expect(&Token::RBrace)?;
                            Ok(Expr::StructLit(name, None, fields))
                        } else {
                            Ok(Expr::TypeIdent(name))
                        }
                    }
                    Token::Dot => {
                        // Type.Variant or Type.method
                        // Don't consume the dot here - let postfix handle it
                        Ok(Expr::TypeIdent(name))
                    }
                    _ => Ok(Expr::TypeIdent(name)),
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                if *self.peek() == Token::Comma {
                    // Tuple literal
                    let mut elements = vec![expr];
                    while *self.peek() == Token::Comma {
                        self.advance();
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::TupleLit(elements))
                } else {
                    // Grouping
                    self.expect(&Token::RParen)?;
                    Ok(expr)
                }
            }
            Token::LBracket => {
                self.advance();
                self.skip_newlines();
                if *self.peek() == Token::RBracket {
                    self.advance();
                    return Ok(Expr::ArrayLit(vec![]));
                }
                let first = self.parse_expr()?;
                self.skip_newlines();
                if *self.peek() == Token::Comma || *self.peek() == Token::RBracket {
                    // Array literal
                    let mut elements = vec![first];
                    while *self.peek() == Token::Comma {
                        self.advance();
                        self.skip_newlines();
                        if *self.peek() == Token::RBracket {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                        self.skip_newlines();
                    }
                    self.expect(&Token::RBracket)?;
                    Ok(Expr::ArrayLit(elements))
                } else {
                    Err(self.error(format!(
                        "expected ',' or ']' in array literal, got {:?}",
                        self.peek()
                    )))
                }
            }
            Token::LBrace => {
                // Could be map literal or block
                // If first thing after { is expr : expr, it's a map
                // If empty {}, it's an empty map
                if self.is_map_lit_ahead() {
                    self.advance(); // {
                    self.skip_newlines();
                    if *self.peek() == Token::RBrace {
                        self.advance();
                        return Ok(Expr::MapLit(vec![]));
                    }
                    let mut entries = Vec::new();
                    loop {
                        self.skip_newlines();
                        let key = self.parse_expr()?;
                        self.expect(&Token::Colon)?;
                        let value = self.parse_expr()?;
                        entries.push((key, value));
                        self.skip_newlines();
                        if *self.peek() == Token::Comma {
                            self.advance();
                            self.skip_newlines();
                        } else {
                            break;
                        }
                    }
                    self.skip_newlines();
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::MapLit(entries))
                } else {
                    let block = self.parse_block()?;
                    Ok(Expr::Block(block))
                }
            }
            Token::If => self.parse_if_expr(),
            Token::Match => self.parse_match_expr(),
            Token::For => self.parse_for_expr(),
            Token::While => self.parse_while_expr(),
            Token::Loop => self.parse_loop_expr(),
            Token::Guard => self.parse_guard_expr(),
            Token::Fn => self.parse_lambda(),
            tok => Err(self.error(format!("unexpected token in expression: {:?}", tok))),
        }
    }

    fn is_struct_lit_ahead(&self) -> bool {
        // Look past { to see if we have ident: pattern (struct lit)
        // or something else (block)
        let mut offset = 1; // past {
                            // Skip newlines
        while let Some(tok) = self.tokens.get(self.pos + offset) {
            if *tok == Token::Newline {
                offset += 1;
            } else {
                break;
            }
        }
        // Check for empty braces
        if let Some(Token::RBrace) = self.tokens.get(self.pos + offset) {
            return false; // empty block, not struct
        }
        // Check for ident:
        if let Some(Token::Ident(_)) = self.tokens.get(self.pos + offset) {
            if let Some(Token::Colon) = self.tokens.get(self.pos + offset + 1) {
                return true;
            }
        }
        false
    }

    fn is_map_lit_ahead(&self) -> bool {
        // Look past { for either } (empty map) or expr : (map entry)
        let mut offset = 1;
        // Skip newlines
        while let Some(tok) = self.tokens.get(self.pos + offset) {
            if *tok == Token::Newline {
                offset += 1;
            } else {
                break;
            }
        }
        if let Some(Token::RBrace) = self.tokens.get(self.pos + offset) {
            return true; // empty {} is a map
        }
        // Look for key: value pattern
        // String literal followed by colon is definitely a map
        match self.tokens.get(self.pos + offset) {
            Some(Token::StringLit(_))
            | Some(Token::IntLit(_))
            | Some(Token::FloatLit(_))
            | Some(Token::True)
            | Some(Token::False) => {
                matches!(self.tokens.get(self.pos + offset + 1), Some(Token::Colon))
            }
            _ => false,
        }
    }

    fn parse_struct_fields(&mut self) -> Result<Vec<(String, Expr)>, String> {
        let mut fields = Vec::new();
        self.skip_newlines();
        if *self.peek() == Token::RBrace {
            return Ok(fields);
        }
        loop {
            self.skip_newlines();
            if *self.peek() == Token::RBrace {
                break;
            }
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            fields.push((name, value));
            self.skip_newlines();
            if *self.peek() == Token::Comma {
                self.advance();
                self.skip_newlines();
            }
        }
        Ok(fields)
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::If)?;
        let cond = self.parse_expr()?;
        self.skip_newlines();
        let then_block = self.parse_block()?;

        let else_expr = if *self.peek() == Token::Else {
            self.advance();
            if *self.peek() == Token::If {
                Some(Box::new(self.parse_if_expr()?))
            } else {
                self.skip_newlines();
                let block = self.parse_block()?;
                Some(Box::new(Expr::Block(block)))
            }
        } else {
            None
        };

        Ok(Expr::If(Box::new(cond), then_block, else_expr))
    }

    fn parse_match_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::Match)?;
        let scrutinee = self.parse_expr()?;
        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while *self.peek() != Token::RBrace {
            self.skip_newlines();
            if *self.peek() == Token::RBrace {
                break;
            }
            let pattern = self.parse_pattern()?;
            self.expect(&Token::FatArrow)?;
            self.skip_newlines();
            let body = if *self.peek() == Token::Return {
                // Return statement in match arm - wrap in a block
                let stmt = self.parse_stmt()?;
                Expr::Block(Block { stmts: vec![stmt] })
            } else if *self.peek() == Token::LBrace {
                let block = self.parse_block()?;
                Expr::Block(block)
            } else {
                self.parse_expr()?
            };

            arms.push(MatchArm { pattern, body });
            self.skip_newlines();
            // Optional comma after arm
            if *self.peek() == Token::Comma {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(&Token::RBrace)?;
        Ok(Expr::Match(Box::new(scrutinee), arms))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match self.peek().clone() {
            Token::Ident(ref name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::IntLit(n) => {
                self.advance();
                Ok(Pattern::IntLit(n))
            }
            Token::FloatLit(f) => {
                self.advance();
                Ok(Pattern::FloatLit(f))
            }
            Token::True => {
                self.advance();
                Ok(Pattern::BoolLit(true))
            }
            Token::False => {
                self.advance();
                Ok(Pattern::BoolLit(false))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Pattern::StringLit(s))
            }
            Token::Nil => {
                self.advance();
                Ok(Pattern::Nil)
            }
            Token::Bang => {
                // Error pattern: !name
                self.advance();
                let name = self.expect_ident()?;
                Ok(Pattern::Error(name))
            }
            Token::Is => {
                // is Type pattern
                self.advance();
                match self.peek().clone() {
                    Token::TypeIdent(name) => {
                        self.advance();
                        if *self.peek() == Token::Dot {
                            self.advance();
                            let variant = self.expect_type_ident()?;
                            Ok(Pattern::IsEnumVariant(name, variant))
                        } else {
                            Ok(Pattern::IsType(TypeExpr::Named(name)))
                        }
                    }
                    Token::Ident(ref name) if is_primitive_type(name) => {
                        let name = name.clone();
                        self.advance();
                        let mut ty = TypeExpr::Named(name);
                        if *self.peek() == Token::Question {
                            self.advance();
                            ty = TypeExpr::Optional(Box::new(ty));
                        }
                        Ok(Pattern::IsType(ty))
                    }
                    tok => Err(self.error(format!("expected type after 'is', got {:?}", tok))),
                }
            }
            Token::TypeIdent(type_name) => {
                self.advance();
                if *self.peek() == Token::Dot {
                    self.advance();
                    let variant = self.expect_type_ident()?;
                    if *self.peek() == Token::LParen {
                        self.advance();
                        let mut bindings = Vec::new();
                        if *self.peek() != Token::RParen {
                            loop {
                                let name = self.expect_ident()?;
                                bindings.push(name);
                                if *self.peek() == Token::Comma {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(&Token::RParen)?;
                        Ok(Pattern::EnumVariant(type_name, variant, bindings))
                    } else {
                        Ok(Pattern::EnumVariant(type_name, variant, vec![]))
                    }
                } else {
                    Err(self.error(format!(
                        "expected '.Variant' after type name in pattern, got {:?}",
                        self.peek()
                    )))
                }
            }
            Token::Ident(name) => {
                self.advance();
                // Check for module.Type.Variant pattern
                if *self.peek() == Token::Dot {
                    if let Token::TypeIdent(type_name) = self.peek_ahead(1).clone() {
                        if *self.peek_ahead(2) == Token::Dot {
                            if let Token::TypeIdent(variant) = self.peek_ahead(3).clone() {
                                self.advance(); // .
                                self.advance(); // TypeIdent
                                self.advance(); // .
                                self.advance(); // Variant
                                if *self.peek() == Token::LParen {
                                    self.advance();
                                    let mut bindings = Vec::new();
                                    if *self.peek() != Token::RParen {
                                        loop {
                                            let bname = self.expect_ident()?;
                                            bindings.push(bname);
                                            if *self.peek() == Token::Comma {
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    self.expect(&Token::RParen)?;
                                    return Ok(Pattern::QualifiedEnumVariant(
                                        name, type_name, variant, bindings,
                                    ));
                                }
                                return Ok(Pattern::QualifiedEnumVariant(
                                    name,
                                    type_name,
                                    variant,
                                    vec![],
                                ));
                            }
                        }
                    }
                }
                Ok(Pattern::Binding(name))
            }
            tok => Err(self.error(format!("unexpected token in pattern: {:?}", tok))),
        }
    }

    fn parse_for_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::For)?;
        let var1 = self.expect_ident()?;
        let var2 = if *self.peek() == Token::Comma {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(&Token::In)?;
        let iterable = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Expr::For(var1, var2, Box::new(iterable), body))
    }

    fn parse_while_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::While)?;
        let cond = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Expr::While(Box::new(cond), body))
    }

    fn parse_loop_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::Loop)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Expr::Loop(body))
    }

    fn parse_guard_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::Guard)?;

        let (binding, condition) = if *self.peek() == Token::Let {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(&Token::Eq)?;
            let expr = self.parse_expr()?;
            (Some(name), expr)
        } else {
            (None, self.parse_expr()?)
        };

        self.expect(&Token::Else)?;
        self.skip_newlines();
        let else_block = self.parse_block()?;

        Ok(Expr::Guard(binding, Box::new(condition), else_block))
    }

    fn parse_lambda(&mut self) -> Result<Expr, String> {
        self.expect(&Token::Fn)?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        let return_type = if *self.peek() == Token::Arrow {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.skip_newlines();
        let body = self.parse_block()?;

        Ok(Expr::Lambda(params, return_type, body))
    }

    fn convert_string_parts(&self, parts: Vec<LexStringPart>) -> Result<Vec<StringPart>, String> {
        let mut result = Vec::new();
        for part in parts {
            match part {
                LexStringPart::Literal(s) => result.push(StringPart::Literal(s)),
                LexStringPart::Expr(tokens) => {
                    let spanned: Vec<SpannedToken> = tokens
                        .into_iter()
                        .map(|t| SpannedToken {
                            token: t,
                            line: 0,
                            col: 0,
                        })
                        .collect();
                    // Add EOF token for the sub-parser
                    let mut all = spanned;
                    all.push(SpannedToken {
                        token: Token::Eof,
                        line: 0,
                        col: 0,
                    });
                    let mut sub_parser = Parser::new(all);
                    let expr = sub_parser.parse_expr()?;
                    result.push(StringPart::Expr(Box::new(expr)));
                }
            }
        }
        Ok(result)
    }
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f64"
            | "bool"
            | "string"
            | "nil"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    fn parse_str(source: &str) -> Result<Program, String> {
        let tokens = lexer::lex(source)?;
        parse(tokens)
    }

    #[test]
    fn test_let_decl() {
        let prog = parse_str("let x = 42").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            Item::Let(decl) => {
                assert_eq!(decl.name, "x");
                assert!(!decl.mutable);
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_let_mut() {
        let prog = parse_str("let mut x = 0").unwrap();
        match &prog.items[0] {
            Item::Let(decl) => {
                assert_eq!(decl.name, "x");
                assert!(decl.mutable);
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_fn_def() {
        let prog = parse_str("fn add(a: i64, b: i64) -> i64 {\n    return a + b\n}").unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
            }
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn test_type_def_struct() {
        let prog = parse_str("type Point {\n    x: f64\n    y: f64\n}").unwrap();
        match &prog.items[0] {
            Item::TypeDef(td) => {
                assert_eq!(td.name, "Point");
                match &td.kind {
                    TypeDefKind::Struct(s) => {
                        assert_eq!(s.fields.len(), 2);
                    }
                    _ => panic!("expected Struct"),
                }
            }
            _ => panic!("expected TypeDef"),
        }
    }

    #[test]
    fn test_type_def_enum() {
        let prog = parse_str("type Color { Red, Green, Blue }").unwrap();
        match &prog.items[0] {
            Item::TypeDef(td) => {
                assert_eq!(td.name, "Color");
                match &td.kind {
                    TypeDefKind::Enum(e) => {
                        assert_eq!(e.variants.len(), 3);
                        assert_eq!(e.variants[0].name, "Red");
                    }
                    _ => panic!("expected Enum"),
                }
            }
            _ => panic!("expected TypeDef"),
        }
    }

    #[test]
    fn test_type_alias() {
        let prog = parse_str("type Numeric = i64 | f64").unwrap();
        match &prog.items[0] {
            Item::TypeDef(td) => {
                assert_eq!(td.name, "Numeric");
                match &td.kind {
                    TypeDefKind::Alias(TypeExpr::Union(members)) => {
                        assert_eq!(members.len(), 2);
                    }
                    _ => panic!("expected Alias union"),
                }
            }
            _ => panic!("expected TypeDef"),
        }
    }

    #[test]
    fn test_if_expr() {
        let prog = parse_str("fn main() {\n    if true {\n        return\n    }\n}").unwrap();
        assert!(matches!(&prog.items[0], Item::Function(_)));
    }

    #[test]
    fn test_array_literal() {
        let prog = parse_str("let arr = [1, 2, 3]").unwrap();
        match &prog.items[0] {
            Item::Let(decl) => match &decl.value {
                Expr::ArrayLit(elems) => assert_eq!(elems.len(), 3),
                _ => panic!("expected ArrayLit"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_map_literal() {
        let prog = parse_str(r#"let m = {"a": 1, "b": 2}"#).unwrap();
        match &prog.items[0] {
            Item::Let(decl) => match &decl.value {
                Expr::MapLit(entries) => assert_eq!(entries.len(), 2),
                _ => panic!("expected MapLit"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_tuple_literal() {
        let prog = parse_str("let t = (1, 2, 3)").unwrap();
        match &prog.items[0] {
            Item::Let(decl) => match &decl.value {
                Expr::TupleLit(elems) => assert_eq!(elems.len(), 3),
                _ => panic!("expected TupleLit"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_match_expr() {
        let src = "fn test(x: i64) -> string {\n    match x {\n        1 => \"one\"\n        2 => \"two\"\n        _ => \"other\"\n    }\n}";
        let prog = parse_str(src).unwrap();
        assert!(matches!(&prog.items[0], Item::Function(_)));
    }

    #[test]
    fn test_for_loop() {
        let src = "fn test() {\n    for i in 0..10 {\n        break\n    }\n}";
        let prog = parse_str(src).unwrap();
        assert!(matches!(&prog.items[0], Item::Function(_)));
    }

    #[test]
    fn test_lambda() {
        let src = "let double = fn(x: i64) -> i64 { return x * 2 }";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Let(decl) => {
                assert!(matches!(&decl.value, Expr::Lambda(_, _, _)));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_precedence() {
        // 2 + 3 * 4 should parse as 2 + (3 * 4)
        let prog = parse_str("let x = 2 + 3 * 4").unwrap();
        match &prog.items[0] {
            Item::Let(decl) => match &decl.value {
                Expr::BinOp(_, BinOp::Add, _) => {} // correct: Add is top-level
                _ => panic!("expected Add at top level"),
            },
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_error_literal() {
        let prog = parse_str(r#"let e = !("error")"#).unwrap();
        match &prog.items[0] {
            Item::Let(decl) => {
                assert!(matches!(&decl.value, Expr::ErrorLit(_)));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_import() {
        let prog = parse_str(r#"import "std/math""#).unwrap();
        match &prog.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.path, "std/math");
            }
            _ => panic!("expected Import"),
        }
    }
}
