use crate::types::*;

pub fn find_captured_vars_in_block(
    body: &TypedBlock,
    params: &[(String, Type)],
) -> std::collections::HashSet<String> {
    let mut result = std::collections::HashSet::new();
    find_lambdas_in_block(body, params, &mut result);
    result
}

fn find_lambdas_in_block(
    block: &TypedBlock,
    outer_params: &[(String, Type)],
    captured: &mut std::collections::HashSet<String>,
) {
    for stmt in &block.stmts {
        find_lambdas_in_stmt(stmt, outer_params, captured);
    }
}

fn find_lambdas_in_stmt(
    stmt: &TypedStmt,
    outer_params: &[(String, Type)],
    captured: &mut std::collections::HashSet<String>,
) {
    match stmt {
        TypedStmt::Let(decl) => find_lambdas_in_expr(&decl.value, outer_params, captured),
        TypedStmt::Assign(_, value) => find_lambdas_in_expr(value, outer_params, captured),
        TypedStmt::Return(Some(e)) | TypedStmt::ReturnError(e) | TypedStmt::Expr(e) => {
            find_lambdas_in_expr(e, outer_params, captured)
        }
        _ => {}
    }
}

fn find_lambdas_in_expr(
    expr: &TypedExpr,
    outer_params: &[(String, Type)],
    captured: &mut std::collections::HashSet<String>,
) {
    match &expr.kind {
        TypedExprKind::Lambda(params, _, body) => {
            // This is a lambda — collect its free variables
            let free = collect_free_vars(body, params);
            // Variables that are free in the lambda AND defined in the outer scope are captured
            for name in &free {
                // Skip global function names — they're resolved by index, not captured
                captured.insert(name.clone());
            }
            // Also recurse into the lambda body for nested lambdas
            find_lambdas_in_block(body, params, captured);
        }
        TypedExprKind::BinOp(l, _, r) | TypedExprKind::Range(l, r) => {
            find_lambdas_in_expr(l, outer_params, captured);
            find_lambdas_in_expr(r, outer_params, captured);
        }
        TypedExprKind::UnaryOp(_, e)
        | TypedExprKind::As(e, _)
        | TypedExprKind::AsSafe(e, _)
        | TypedExprKind::Try(e)
        | TypedExprKind::ErrorLit(e) => find_lambdas_in_expr(e, outer_params, captured),
        TypedExprKind::Call(f, args) => {
            find_lambdas_in_expr(f, outer_params, captured);
            for a in args {
                find_lambdas_in_expr(a, outer_params, captured);
            }
        }
        TypedExprKind::MethodCall(obj, _, args) => {
            find_lambdas_in_expr(obj, outer_params, captured);
            for a in args {
                find_lambdas_in_expr(a, outer_params, captured);
            }
        }
        TypedExprKind::If(cond, then_b, else_e) => {
            find_lambdas_in_expr(cond, outer_params, captured);
            find_lambdas_in_block(then_b, outer_params, captured);
            if let Some(e) = else_e {
                find_lambdas_in_expr(e, outer_params, captured);
            }
        }
        TypedExprKind::Block(b) => find_lambdas_in_block(b, outer_params, captured),
        TypedExprKind::For(_, _, iter, body) => {
            find_lambdas_in_expr(iter, outer_params, captured);
            find_lambdas_in_block(body, outer_params, captured);
        }
        TypedExprKind::While(cond, body) | TypedExprKind::Guard(_, cond, body) => {
            find_lambdas_in_expr(cond, outer_params, captured);
            find_lambdas_in_block(body, outer_params, captured);
        }
        TypedExprKind::Loop(body) => find_lambdas_in_block(body, outer_params, captured),
        TypedExprKind::Match(scrut, arms) => {
            find_lambdas_in_expr(scrut, outer_params, captured);
            for arm in arms {
                find_lambdas_in_expr(&arm.body, outer_params, captured);
            }
        }
        TypedExprKind::ArrayLit(elems) | TypedExprKind::TupleLit(elems) => {
            for e in elems {
                find_lambdas_in_expr(e, outer_params, captured);
            }
        }
        TypedExprKind::StructLit(_, fields) => {
            for (_, e) in fields {
                find_lambdas_in_expr(e, outer_params, captured);
            }
        }
        _ => {}
    }
}

/// Collect free variables in a typed block — identifiers that are not
/// locally defined within the block or its sub-blocks.
pub fn collect_free_vars(body: &TypedBlock, params: &[(String, Type)]) -> Vec<String> {
    let mut free = Vec::new();
    let mut defined: std::collections::HashSet<String> =
        params.iter().map(|(n, _)| n.clone()).collect();
    collect_free_in_block(body, &mut defined, &mut free);
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    free.retain(|n| seen.insert(n.clone()));
    free
}

fn collect_free_in_block(
    block: &TypedBlock,
    defined: &mut std::collections::HashSet<String>,
    free: &mut Vec<String>,
) {
    for stmt in &block.stmts {
        collect_free_in_stmt(stmt, defined, free);
    }
}

fn collect_free_in_stmt(
    stmt: &TypedStmt,
    defined: &mut std::collections::HashSet<String>,
    free: &mut Vec<String>,
) {
    match stmt {
        TypedStmt::Let(decl) => {
            collect_free_in_expr(&decl.value, defined, free);
            defined.insert(decl.name.clone());
        }
        TypedStmt::LetTuple(bindings, value) => {
            collect_free_in_expr(value, defined, free);
            for (name, _) in bindings {
                defined.insert(name.clone());
            }
        }
        TypedStmt::Assign(target, value) => {
            match target.as_ref() {
                TypedAssignTarget::Variable(name, _) => {
                    if !defined.contains(name) {
                        free.push(name.clone());
                    }
                }
                TypedAssignTarget::Field(obj, _, _) => collect_free_in_expr(obj, defined, free),
                TypedAssignTarget::Index(obj, idx, _) => {
                    collect_free_in_expr(obj, defined, free);
                    collect_free_in_expr(idx, defined, free);
                }
            }
            collect_free_in_expr(value, defined, free);
        }
        TypedStmt::Return(Some(e)) | TypedStmt::ReturnError(e) | TypedStmt::Expr(e) => {
            collect_free_in_expr(e, defined, free);
        }
        TypedStmt::Return(None) | TypedStmt::Break | TypedStmt::Continue => {}
    }
}

fn collect_free_in_expr(
    expr: &TypedExpr,
    defined: &mut std::collections::HashSet<String>,
    free: &mut Vec<String>,
) {
    match &expr.kind {
        TypedExprKind::Ident(name) => {
            if !defined.contains(name) {
                free.push(name.clone());
            }
        }
        TypedExprKind::BinOp(l, _, r) => {
            collect_free_in_expr(l, defined, free);
            collect_free_in_expr(r, defined, free);
        }
        TypedExprKind::UnaryOp(_, e)
        | TypedExprKind::As(e, _)
        | TypedExprKind::AsSafe(e, _)
        | TypedExprKind::Is(e, _)
        | TypedExprKind::Try(e)
        | TypedExprKind::ErrorLit(e) => {
            collect_free_in_expr(e, defined, free);
        }
        TypedExprKind::Call(f, args) => {
            collect_free_in_expr(f, defined, free);
            for a in args {
                collect_free_in_expr(a, defined, free);
            }
        }
        TypedExprKind::MethodCall(obj, _, args) => {
            collect_free_in_expr(obj, defined, free);
            for a in args {
                collect_free_in_expr(a, defined, free);
            }
        }
        TypedExprKind::StaticMethodCall(_, _, args) => {
            for a in args {
                collect_free_in_expr(a, defined, free);
            }
        }
        TypedExprKind::FieldAccess(e, _) | TypedExprKind::Index(e, _) => {
            collect_free_in_expr(e, defined, free);
        }
        TypedExprKind::ArrayLit(elems) | TypedExprKind::TupleLit(elems) => {
            for e in elems {
                collect_free_in_expr(e, defined, free);
            }
        }
        TypedExprKind::MapLit(entries) => {
            for (k, v) in entries {
                collect_free_in_expr(k, defined, free);
                collect_free_in_expr(v, defined, free);
            }
        }
        TypedExprKind::StructLit(_, fields) => {
            for (_, e) in fields {
                collect_free_in_expr(e, defined, free);
            }
        }
        TypedExprKind::EnumVariant(_, _, args) | TypedExprKind::Panic(args) => {
            for a in args {
                collect_free_in_expr(a, defined, free);
            }
        }
        TypedExprKind::If(cond, then_b, else_e) => {
            collect_free_in_expr(cond, defined, free);
            collect_free_in_block(then_b, defined, free);
            if let Some(e) = else_e {
                collect_free_in_expr(e, defined, free);
            }
        }
        TypedExprKind::Match(scrut, arms) => {
            collect_free_in_expr(scrut, defined, free);
            for arm in arms {
                collect_free_in_expr(&arm.body, defined, free);
            }
        }
        TypedExprKind::For(v1, v2, iter, body) => {
            collect_free_in_expr(iter, defined, free);
            defined.insert(v1.clone());
            if let Some(v) = v2 {
                defined.insert(v.clone());
            }
            collect_free_in_block(body, defined, free);
        }
        TypedExprKind::While(cond, body) => {
            collect_free_in_expr(cond, defined, free);
            collect_free_in_block(body, defined, free);
        }
        TypedExprKind::Loop(body) | TypedExprKind::Block(body) => {
            collect_free_in_block(body, defined, free);
        }
        TypedExprKind::Guard(binding, e, else_b) => {
            collect_free_in_expr(e, defined, free);
            collect_free_in_block(else_b, defined, free);
            if let Some(n) = binding {
                defined.insert(n.clone());
            }
        }
        TypedExprKind::Lambda(params, _, body) => {
            let mut inner_defined = defined.clone();
            for (n, _) in params {
                inner_defined.insert(n.clone());
            }
            collect_free_in_block(body, &mut inner_defined, free);
        }
        TypedExprKind::Range(s, e) => {
            collect_free_in_expr(s, defined, free);
            collect_free_in_expr(e, defined, free);
        }
        TypedExprKind::InterpolatedString(parts) => {
            for p in parts {
                if let TypedStringPart::Expr(e) = p {
                    collect_free_in_expr(e, defined, free);
                }
            }
        }
        _ => {} // Literals, Nil, etc.
    }
}
