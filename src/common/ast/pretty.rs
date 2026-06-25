use crate::common::ast::{
    ast::{Program, QualifierType, Type},
    decl::Decl,
    expr::{BinOp, Expr, Literal, MemberAccess, PostfixOp, PrefixOp, UnOp},
    stmt::{Stmt, SwitchLabel},
};

pub fn pretty_program(program: &Program) -> String {
    let mut out = String::from("Program\n");
    let n = program.decls.len();
    for (i, decl) in program.decls.iter().enumerate() {
        fmt_decl(&mut out, decl, "", i == n - 1);
    }
    out
}

fn branch(prefix: &str, is_last: bool) -> (&'static str, String) {
    if is_last {
        ("└── ", format!("{}    ", prefix))
    } else {
        ("├── ", format!("{}│   ", prefix))
    }
}

fn fmt_decl(out: &mut String, decl: &Decl, prefix: &str, is_last: bool) {
    let (conn, child_pfx) = branch(prefix, is_last);
    match decl {
        Decl::Function(ret, name, params, body, _) => {
            let params_str = if params.is_empty() {
                "[]".into()
            } else {
                let p: Vec<_> = params
                    .iter()
                    .map(|(qty, n)| format!("{} {}", fmt_qty(qty), n))
                    .collect();
                format!("[{}]", p.join(", "))
            };
            out.push_str(&format!(
                "{}{}FunctionDecl({} {}, params={})\n",
                prefix,
                conn,
                fmt_qty(ret),
                name,
                params_str
            ));
            let m = body.len();
            for (j, stmt) in body.iter().enumerate() {
                fmt_stmt(out, stmt, &child_pfx, j == m - 1);
            }
        }
        Decl::GlobalVar(qty, name, init, _) => {
            if let Some(expr) = init {
                out.push_str(&format!(
                    "{}{}GlobalVar({} {} =)\n",
                    prefix,
                    conn,
                    fmt_qty(qty),
                    name
                ));
                fmt_expr(out, expr, &child_pfx, true);
            } else {
                out.push_str(&format!(
                    "{}{}GlobalVar({} {})\n",
                    prefix,
                    conn,
                    fmt_qty(qty),
                    name
                ));
            }
        }
        Decl::StructDecl(name, fields, _) => {
            out.push_str(&format!("{}{}StructDecl({})\n", prefix, conn, name));
            let m = fields.len();
            for (j, (qty, fname)) in fields.iter().enumerate() {
                let (fc, _) = branch(&child_pfx, j == m - 1);
                out.push_str(&format!(
                    "{}{}Field({} {})\n",
                    child_pfx,
                    fc,
                    fmt_qty(qty),
                    fname
                ));
            }
        }
        Decl::EnumDecl(name, variants, _) => {
            out.push_str(&format!("{}{}EnumDecl({})\n", prefix, conn, name));
            let m = variants.len();
            for (j, (vname, val)) in variants.iter().enumerate() {
                let (vc, vc_pfx) = branch(&child_pfx, j == m - 1);
                if let Some(expr) = val {
                    out.push_str(&format!("{}{}Variant({} =)\n", child_pfx, vc, vname));
                    fmt_expr(out, expr, &vc_pfx, true);
                } else {
                    out.push_str(&format!("{}{}Variant({})\n", child_pfx, vc, vname));
                }
            }
        }
        Decl::Typedef(qty, name, _) => {
            out.push_str(&format!(
                "{}{}Typedef({} = {})\n",
                prefix,
                conn,
                name,
                fmt_qty(qty)
            ));
        }
        Decl::Prototype(ret, name, params, _) => {
            let params_str = if params.is_empty() {
                "[]".into()
            } else {
                let p: Vec<_> = params
                    .iter()
                    .map(|(qty, n)| format!("{} {}", fmt_qty(qty), n))
                    .collect();
                format!("[{}]", p.join(", "))
            };
            out.push_str(&format!(
                "{}{}Prototype({} {}, params={})\n",
                prefix,
                conn,
                fmt_qty(ret),
                name,
                params_str
            ));
        }
    }
}

fn fmt_stmt(out: &mut String, stmt: &Stmt, prefix: &str, is_last: bool) {
    let (conn, child_pfx) = branch(prefix, is_last);
    match stmt {
        Stmt::VarDecl(qty, name, init, _) => {
            if let Some(expr) = init {
                out.push_str(&format!(
                    "{}{}VarDecl({} {} =)\n",
                    prefix,
                    conn,
                    fmt_qty(qty),
                    name
                ));
                fmt_expr(out, expr, &child_pfx, true);
            } else {
                out.push_str(&format!(
                    "{}{}VarDecl({} {})\n",
                    prefix,
                    conn,
                    fmt_qty(qty),
                    name
                ));
            }
        }
        Stmt::ExprStmt(expr, _) => {
            out.push_str(&format!("{}{}ExprStmt\n", prefix, conn));
            fmt_expr(out, expr, &child_pfx, true);
        }
        Stmt::Return(expr, _) => {
            out.push_str(&format!("{}{}Return\n", prefix, conn));
            if let Some(e) = expr {
                fmt_expr(out, e, &child_pfx, true);
            }
        }
        Stmt::Block(stmts, _) => {
            out.push_str(&format!("{}{}Block\n", prefix, conn));
            let m = stmts.len();
            for (j, s) in stmts.iter().enumerate() {
                fmt_stmt(out, s, &child_pfx, j == m - 1);
            }
        }
        Stmt::If(cond, then, else_, _) => {
            out.push_str(&format!("{}{}If\n", prefix, conn));
            let has_else = else_.is_some();
            fmt_expr(out, cond, &child_pfx, false);
            if has_else {
                fmt_stmt(out, then, &child_pfx, false);
                fmt_stmt(out, else_.as_ref().unwrap(), &child_pfx, true);
            } else {
                fmt_stmt(out, then, &child_pfx, true);
            }
        }
        Stmt::While(cond, body, _) => {
            out.push_str(&format!("{}{}While\n", prefix, conn));
            fmt_expr(out, cond, &child_pfx, false);
            fmt_stmt(out, body, &child_pfx, true);
        }
        Stmt::DoWhile(cond, body, _) => {
            out.push_str(&format!("{}{}DoWhile\n", prefix, conn));
            fmt_stmt(out, body, &child_pfx, false);
            fmt_expr(out, cond, &child_pfx, true);
        }
        Stmt::For(init, cond, inc, body, _) => {
            out.push_str(&format!("{}{}For\n", prefix, conn));
            let children_count =
                init.is_some() as usize + cond.is_some() as usize + inc.is_some() as usize + 1;
            let mut remaining = children_count;
            if let Some(s) = init {
                remaining -= 1;
                fmt_stmt(out, s, &child_pfx, remaining == 0);
            }
            if let Some(e) = cond {
                remaining -= 1;
                fmt_expr(out, e, &child_pfx, remaining == 0);
            }
            if let Some(e) = inc {
                remaining -= 1;
                fmt_expr(out, e, &child_pfx, remaining == 0);
            }
            fmt_stmt(out, body, &child_pfx, true);
        }
        Stmt::Switch(expr, cases, _) => {
            out.push_str(&format!("{}{}Switch\n", prefix, conn));
            fmt_expr(out, expr, &child_pfx, cases.is_empty());
            let m = cases.len();
            for (j, case) in cases.iter().enumerate() {
                let (cc, cc_pfx) = branch(&child_pfx, j == m - 1);
                match &case.label {
                    SwitchLabel::Case(e) => {
                        out.push_str(&format!("{}{}Case\n", child_pfx, cc));
                        fmt_expr(out, e, &cc_pfx, case.stmts.is_empty());
                    }
                    SwitchLabel::Default => {
                        out.push_str(&format!("{}{}Default\n", child_pfx, cc));
                    }
                }
                let sm = case.stmts.len();
                for (k, s) in case.stmts.iter().enumerate() {
                    fmt_stmt(out, s, &cc_pfx, k == sm - 1);
                }
            }
        }
        Stmt::Break(_) => out.push_str(&format!("{}{}Break\n", prefix, conn)),
        Stmt::Continue(_) => out.push_str(&format!("{}{}Continue\n", prefix, conn)),
    }
}

fn fmt_expr(out: &mut String, expr: &Expr, prefix: &str, is_last: bool) {
    let (conn, child_pfx) = branch(prefix, is_last);
    match expr {
        Expr::Literal(lit, _) => {
            let s = match lit {
                Literal::Int(v) => format!("Literal(Int({}))", v),
                Literal::Double(v) => format!("Literal(Double({}))", v),
                Literal::Char(c) => format!("Literal(Char({:?}))", c),
                Literal::String(s) => format!("Literal(String({:?}))", s),
            };
            out.push_str(&format!("{}{}{}\n", prefix, conn, s));
        }
        Expr::Ident(name, _) => {
            out.push_str(&format!("{}{}Ident({})\n", prefix, conn, name));
        }
        Expr::Binary(lhs, op, rhs, _) => {
            out.push_str(&format!("{}{}Binary({})\n", prefix, conn, fmt_binop(op)));
            fmt_expr(out, lhs, &child_pfx, false);
            fmt_expr(out, rhs, &child_pfx, true);
        }
        Expr::Unary(op, operand, _) => {
            let s = match op {
                UnOp::Neg => "Unary(-)",
                UnOp::Not => "Unary(!)",
                UnOp::BitNot => "Unary(~)",
                UnOp::Deref => "Unary(*)",
                UnOp::AddrOf => "Unary(&)",
            };
            out.push_str(&format!("{}{}{}\n", prefix, conn, s));
            fmt_expr(out, operand, &child_pfx, true);
        }
        Expr::Prefix(op, operand, _) => {
            let s = match op {
                PrefixOp::Inc => "Prefix(++)",
                PrefixOp::Dec => "Prefix(--)",
            };
            out.push_str(&format!("{}{}{}\n", prefix, conn, s));
            fmt_expr(out, operand, &child_pfx, true);
        }
        Expr::Postfix(op, operand, _) => {
            let s = match op {
                PostfixOp::Inc => "Postfix(++)",
                PostfixOp::Dec => "Postfix(--)",
            };
            out.push_str(&format!("{}{}{}\n", prefix, conn, s));
            fmt_expr(out, operand, &child_pfx, true);
        }
        Expr::Call(callee, args, _) => {
            out.push_str(&format!("{}{}Call\n", prefix, conn));
            fmt_expr(out, callee, &child_pfx, args.is_empty());
            let m = args.len();
            for (j, arg) in args.iter().enumerate() {
                fmt_expr(out, arg, &child_pfx, j == m - 1);
            }
        }
        Expr::Cast(qty, expr, _) => {
            out.push_str(&format!("{}{}Cast({})\n", prefix, conn, fmt_qty(qty)));
            fmt_expr(out, expr, &child_pfx, true);
        }
        Expr::Index(arr, idx, _) => {
            out.push_str(&format!("{}{}Index\n", prefix, conn));
            fmt_expr(out, arr, &child_pfx, false);
            fmt_expr(out, idx, &child_pfx, true);
        }
        Expr::Assign(lhs, rhs, _) => {
            out.push_str(&format!("{}{}Assign\n", prefix, conn));
            fmt_expr(out, lhs, &child_pfx, false);
            fmt_expr(out, rhs, &child_pfx, true);
        }
        Expr::CompoundAssign(op, lhs, rhs, _) => {
            out.push_str(&format!(
                "{}{}CompoundAssign({}=)\n",
                prefix,
                conn,
                fmt_binop(op)
            ));
            fmt_expr(out, lhs, &child_pfx, false);
            fmt_expr(out, rhs, &child_pfx, true);
        }
        Expr::Sizeof(inner, _) => {
            out.push_str(&format!("{}{}Sizeof\n", prefix, conn));
            fmt_expr(out, inner, &child_pfx, true);
        }
        Expr::SizeofType(qty, _) => {
            out.push_str(&format!("{}{}SizeofType({})\n", prefix, conn, fmt_qty(qty)));
        }
        Expr::Ternary(cond, then, else_, _) => {
            out.push_str(&format!("{}{}Ternary\n", prefix, conn));
            fmt_expr(out, cond, &child_pfx, false);
            fmt_expr(out, then, &child_pfx, false);
            fmt_expr(out, else_, &child_pfx, true);
        }
        Expr::Member(obj, access, field, _) => {
            let op = match access {
                MemberAccess::Direct => ".",
                MemberAccess::Pointer => "->",
            };
            out.push_str(&format!("{}{}Member({}{})\n", prefix, conn, op, field));
            fmt_expr(out, obj, &child_pfx, true);
        }
    }
}

fn fmt_qty(qty: &QualifierType) -> String {
    let mut s = String::new();
    if qty.is_const {
        s.push_str("const ");
    }
    if qty.is_unsigned {
        s.push_str("unsigned ");
    }
    s.push_str(&fmt_type(&qty.ty));
    s
}

fn fmt_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".into(),
        Type::Long => "long".into(),
        Type::Short => "short".into(),
        Type::Char => "char".into(),
        Type::Float => "float".into(),
        Type::Double => "double".into(),
        Type::Void => "void".into(),
        Type::Pointer(inner) => format!("{}*", fmt_type(inner)),
        Type::Array(inner, size) => match size {
            Some(size) => format!("{}[{size}]", fmt_type(inner)),
            None => format!("{}[]", fmt_type(inner)),
        },
        Type::Struct(n) => format!("struct {}", n),
        Type::Enum(n) => format!("enum {}", n),
        Type::Alias(n) => n.clone(),
        Type::Function(ret, params) => {
            let params_str = params.iter().map(fmt_qty).collect::<Vec<_>>().join(", ");
            format!("fn({}) -> {}", params_str, fmt_qty(ret))
        }
    }
}

fn fmt_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Neq => "!=",
        BinOp::Less => "<",
        BinOp::Greater => ">",
        BinOp::Leq => "<=",
        BinOp::Geq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}
