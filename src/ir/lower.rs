use crate::common::ast::{
    ast::{Program, QualifierType, Type},
    decl::Decl,
    expr::{BinOp, Expr, Literal, MemberAccess, PostfixOp, PrefixOp, UnOp},
    stmt::{Stmt, SwitchLabel},
};
use crate::common::errors::types::CodegenError;
use crate::ir::tac::{
    ConstValue, LabelGen, LabelId, Operand, TacFunction, TacInstr, TacProgram, TempGen, TempId,
};
use std::collections::HashMap;

type LowerResult<T> = Result<T, CodegenError>;

/// Layout calculado de uma struct: offset (em bytes, a partir do endereco
/// base da struct) e tipo resolvido de cada campo, mais o tamanho total
/// (arredondado para a maior alinhamento entre os campos).
#[derive(Debug, Clone)]
struct StructLayout {
    fields: Vec<(String, i64, Type)>,
    size: i64,
}

#[derive(Debug, Clone)]
pub struct Lowerer {
    temps: TempGen,
    labels: LabelGen,
    instrs: Vec<TacInstr>,
    /// Tipo declarado de cada variavel/parametro visto ate agora na funcao
    /// atual. Usado para resolver `sizeof(expr)`, indexacao via ponteiro e
    /// acesso a membro; nao substitui a analise semantica (que ja validou o
    /// programa antes do lowering).
    var_types: HashMap<String, Type>,
    /// Layout (offsets + tamanho) de cada struct declarada no programa,
    /// calculado uma vez em `lower_program` e compartilhado entre as
    /// funcoes. Vazio quando o `Lowerer` e usado isoladamente (ex.: testes
    /// unitarios deste modulo via `Lowerer::new()`).
    struct_layouts: HashMap<String, StructLayout>,
    /// Tabela de `typedef`: nome do alias -> tipo subjacente (ainda podendo
    /// ser outro alias, resolvido por `resolve_alias`).
    typedefs: HashMap<String, Type>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ControlLabels {
    break_label: Option<LabelId>,
    continue_label: Option<LabelId>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self::with_context(HashMap::new(), HashMap::new())
    }

    fn with_context(
        struct_layouts: HashMap<String, StructLayout>,
        typedefs: HashMap<String, Type>,
    ) -> Self {
        Self {
            temps: TempGen::new(),
            labels: LabelGen::new(),
            instrs: Vec::new(),
            var_types: HashMap::new(),
            struct_layouts,
            typedefs,
        }
    }

    /// Registra o tipo declarado de `name`, usado depois para resolver
    /// `sizeof(name)`. Chamado para parametros de funcao e para cada
    /// `VarDecl` conforme o lowering avanca.
    fn declare_var_type(&mut self, name: &str, ty: &Type) {
        self.var_types.insert(name.to_string(), ty.clone());
    }

    /// Tamanho (em bytes) de cada variavel cujo valor nao cabe num slot
    /// escalar de 8 bytes — structs locais e arrays fixos. Chamado ao final do
    /// lowering de uma funcao, para popular `TacFunction::var_sizes`.
    fn compute_var_sizes(&self) -> HashMap<String, i64> {
        let mut sizes = HashMap::new();
        for (name, ty) in &self.var_types {
            match resolve_alias(ty, &self.typedefs) {
                Type::Struct(struct_name) => {
                    if let Some(layout) = self.struct_layouts.get(&struct_name) {
                        sizes.insert(name.clone(), layout.size);
                    }
                }
                array_ty @ Type::Array(_, _) => {
                    if let Ok(size) = type_size(&array_ty) {
                        sizes.insert(name.clone(), size);
                    }
                }
                _ => {}
            }
        }
        sizes
    }

    /// Infere o tipo estatico (ja resolvido de aliases de `typedef`) de um
    /// subconjunto limitado de expressoes (identificadores, deref, indice
    /// via ponteiro, membro de struct e cast) — o suficiente para resolver
    /// o tamanho do elemento em `arr[i]` e o offset de campo em `s.campo`.
    /// Nao substitui a analise semantica completa; expressoes fora desse
    /// subconjunto retornam um erro explicito em vez de um palpite.
    fn infer_type(&self, expr: &Expr) -> LowerResult<Type> {
        match expr {
            Expr::Ident(name, _) => self
                .var_types
                .get(name)
                .map(|ty| resolve_alias(ty, &self.typedefs))
                .ok_or_else(|| codegen_error("tipo de variavel desconhecido no lowering", Some("type"))),
            Expr::Unary(UnOp::Deref, inner, _) => match self.infer_type(inner)? {
                Type::Pointer(t) | Type::Array(t, _) => Ok(resolve_alias(&t, &self.typedefs)),
                _ => Err(codegen_error(
                    "deref de valor que nao e ponteiro/array",
                    Some("type"),
                )),
            },
            Expr::Index(arr, _, _) => match self.infer_type(arr)? {
                Type::Pointer(t) => Ok(resolve_alias(&t, &self.typedefs)),
                Type::Array(t, Some(_)) => Ok(resolve_alias(&t, &self.typedefs)),
                Type::Array(_, None) => Err(codegen_error(
                    "indexacao de array com tamanho desconhecido nao suportada no lowering",
                    Some("index"),
                )),
                _ => Err(codegen_error(
                    "indexacao de valor que nao e ponteiro/array",
                    Some("index"),
                )),
            },
            Expr::Member(obj, access, field, _) => {
                let layout = self.struct_layout_of_member_base(obj, access)?;
                layout
                    .fields
                    .iter()
                    .find(|(name, _, _)| name == field)
                    .map(|(_, _, ty)| ty.clone())
                    .ok_or_else(|| {
                        codegen_error("campo de struct desconhecido no lowering", Some("member"))
                    })
            }
            Expr::Cast(qty, _, _) => Ok(resolve_alias(&qty.ty, &self.typedefs)),
            _ => Err(codegen_error(
                "tipo de expressao nao inferido no lowering (suporte limitado a identificador, deref, indice, membro e cast)",
                Some("type"),
            )),
        }
    }

    /// Resolve o layout da struct base de um acesso a membro: para `.`,
    /// `obj` deve ser a propria struct; para `->`, `obj` deve ser um
    /// ponteiro para struct.
    fn struct_layout_of_member_base(
        &self,
        obj: &Expr,
        access: &MemberAccess,
    ) -> LowerResult<&StructLayout> {
        let struct_name = match access {
            MemberAccess::Direct => match self.infer_type(obj)? {
                Type::Struct(name) => name,
                _ => {
                    return Err(codegen_error(
                        "acesso '.' em valor que nao e struct",
                        Some("member"),
                    ))
                }
            },
            MemberAccess::Pointer => match self.infer_type(obj)? {
                Type::Pointer(inner) => match resolve_alias(&inner, &self.typedefs) {
                    Type::Struct(name) => name,
                    _ => {
                        return Err(codegen_error(
                            "acesso '->' em ponteiro que nao e para struct",
                            Some("member"),
                        ))
                    }
                },
                _ => {
                    return Err(codegen_error(
                        "acesso '->' em valor que nao e ponteiro",
                        Some("member"),
                    ))
                }
            },
        };
        self.struct_layouts.get(&struct_name).ok_or_else(|| {
            codegen_error(
                "layout de struct desconhecido no lowering (campo agregado aninhado nao suportado, ou struct nunca declarada)",
                Some("member"),
            )
        })
    }

    /// Calcula o endereco (em bytes) de `arr[idx]`.
    ///
    /// Para ponteiros, a base e o valor do ponteiro. Para arrays fixos, a
    /// base e o endereco do bloco contiguo reservado para a variavel.
    fn lower_index_address(&mut self, arr: &Expr, idx: &Expr) -> LowerResult<Operand> {
        let (elem_ty, base_ptr) = match self.infer_type(arr)? {
            Type::Pointer(inner) => (resolve_alias(&inner, &self.typedefs), self.lower_expr(arr)?),
            Type::Array(inner, Some(_)) => (
                resolve_alias(&inner, &self.typedefs),
                self.lower_address_of(arr)?,
            ),
            Type::Array(_, None) => {
                return Err(codegen_error(
                    "indexacao de array com tamanho desconhecido nao suportada no lowering",
                    Some("index"),
                ))
            }
            _ => {
                return Err(codegen_error(
                    "indexacao de valor que nao e ponteiro/array",
                    Some("index"),
                ))
            }
        };
        let elem_size = type_size(&elem_ty)?;
        let idx_op = self.lower_expr(idx)?;

        let offset = if elem_size == 1 {
            idx_op
        } else {
            let scaled = self.fresh_temp();
            self.instrs.push(TacInstr::BinOp {
                dst: scaled,
                op: BinOp::Mul,
                lhs: idx_op,
                rhs: Operand::Const(ConstValue::Int(elem_size)),
            });
            Operand::Temp(scaled)
        };

        let addr = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: addr,
            op: BinOp::Add,
            lhs: base_ptr,
            rhs: offset,
        });
        Ok(Operand::Temp(addr))
    }

    /// Calcula o endereco (em bytes) de `obj.campo` ou `obj->campo`:
    /// endereco da struct base + offset do campo no layout.
    fn lower_member_address(
        &mut self,
        obj: &Expr,
        access: &MemberAccess,
        field: &str,
    ) -> LowerResult<Operand> {
        let field_offset = {
            let layout = self.struct_layout_of_member_base(obj, access)?;
            layout
                .fields
                .iter()
                .find(|(name, _, _)| name == field)
                .map(|(_, offset, _)| *offset)
                .ok_or_else(|| {
                    codegen_error("campo de struct desconhecido no lowering", Some("member"))
                })?
        };

        let base_addr = match access {
            MemberAccess::Direct => self.lower_address_of(obj)?,
            MemberAccess::Pointer => self.lower_expr(obj)?,
        };

        if field_offset == 0 {
            return Ok(base_addr);
        }

        let addr = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: addr,
            op: BinOp::Add,
            lhs: base_addr,
            rhs: Operand::Const(ConstValue::Int(field_offset)),
        });
        Ok(Operand::Temp(addr))
    }

    /// Calcula o *endereco* (nao o valor) de uma expressao-lvalue:
    /// identificador, `*p` (endereco = o proprio `p`), `arr[i]` ou
    /// `obj.campo`/`obj->campo`.
    fn lower_address_of(&mut self, expr: &Expr) -> LowerResult<Operand> {
        match expr {
            Expr::Ident(name, _) => {
                let temp = self.fresh_temp();
                self.instrs.push(TacInstr::UnOp {
                    dst: temp,
                    op: UnOp::AddrOf,
                    src: Operand::Var(name.clone()),
                });
                Ok(Operand::Temp(temp))
            }
            Expr::Unary(UnOp::Deref, inner, _) => self.lower_expr(inner),
            Expr::Index(arr, idx, _) => self.lower_index_address(arr, idx),
            Expr::Member(obj, access, field, _) => self.lower_member_address(obj, access, field),
            _ => Err(codegen_error(
                "nao e possivel obter o endereco desta expressao no lowering",
                Some("addr"),
            )),
        }
    }

    pub fn lower_expr(&mut self, expr: &Expr) -> LowerResult<Operand> {
        match expr {
            Expr::Literal(value, _) => Ok(Operand::Const(lower_literal(value))),
            Expr::Ident(name, _) => Ok(Operand::Var(name.clone())),
            Expr::Binary(lhs, op, rhs, _) => {
                let lhs = self.lower_expr(lhs)?;
                let rhs = self.lower_expr(rhs)?;
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::BinOp {
                    dst,
                    op: op.clone(),
                    lhs,
                    rhs,
                });
                Ok(Operand::Temp(dst))
            }
            Expr::Unary(op, src, _) => {
                let src = self.lower_expr(src)?;
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::UnOp {
                    dst,
                    op: op.clone(),
                    src,
                });
                Ok(Operand::Temp(dst))
            }
            Expr::Prefix(op, target, _) => self.lower_prefix(op, target),
            Expr::Postfix(op, target, _) => self.lower_postfix(op, target),
            Expr::Call(callee, args, _) => {
                let fn_name = match callee.as_ref() {
                    Expr::Ident(name, _) => name.clone(),
                    _ => {
                        return Err(codegen_error(
                            "chamada por expressao nao suportada no lowering",
                            Some("call"),
                        ));
                    }
                };
                let mut lowered_args = Vec::with_capacity(args.len());
                for arg in args {
                    lowered_args.push(self.lower_expr(arg)?);
                }
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::Call {
                    dst: Some(dst),
                    fn_name,
                    args: lowered_args,
                });
                Ok(Operand::Temp(dst))
            }
            Expr::Cast(_, inner, _) => self.lower_expr(inner),
            Expr::Assign(lhs, rhs, _) => {
                let src = self.lower_expr(rhs)?;
                let dst = self.lower_assignment_target(lhs)?;
                self.emit_copy(dst.clone(), src)?;
                Ok(dst)
            }
            Expr::CompoundAssign(op, lhs, rhs, _) => {
                let dst = self.lower_assignment_target(lhs)?;
                let rhs = self.lower_expr(rhs)?;
                let temp = self.fresh_temp();
                self.instrs.push(TacInstr::BinOp {
                    dst: temp,
                    op: op.clone(),
                    lhs: dst.clone(),
                    rhs,
                });
                self.emit_copy(dst.clone(), Operand::Temp(temp))?;
                Ok(dst)
            }
            Expr::SizeofType(qty, _) => Ok(Operand::Const(ConstValue::Int(type_size(&qty.ty)?))),
            Expr::Ternary(cond, then_expr, else_expr, _) => {
                let cond = self.lower_expr(cond)?;
                let then_label = self.labels.fresh();
                let else_label = self.labels.fresh();
                let end_label = self.labels.fresh();
                let dst = self.fresh_temp();

                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label,
                    else_label,
                });

                self.instrs.push(TacInstr::Label(then_label));
                let then_val = self.lower_expr(then_expr)?;
                self.emit_copy(Operand::Temp(dst), then_val)?;
                self.emit_jump_unless_terminated(end_label);

                self.instrs.push(TacInstr::Label(else_label));
                let else_val = self.lower_expr(else_expr)?;
                self.emit_copy(Operand::Temp(dst), else_val)?;

                self.instrs.push(TacInstr::Label(end_label));
                Ok(Operand::Temp(dst))
            }
            Expr::Index(arr, idx, _) => {
                let addr = self.lower_index_address(arr, idx)?;
                Ok(Operand::Deref(Box::new(addr)))
            }
            Expr::Member(obj, access, field, _) => {
                let addr = self.lower_member_address(obj, access, field)?;
                Ok(Operand::Deref(Box::new(addr)))
            }
            // `sizeof(expr)`: o caso pratico mais comum e `sizeof(variavel)`.
            // O tipo declarado de identificadores e rastreado em
            // `var_types` (preenchido a partir de parametros e `VarDecl`);
            // para qualquer outra forma de expressao ainda nao ha
            // informacao de tipo disponivel no lowering.
            Expr::Sizeof(inner, _) => match inner.as_ref() {
                Expr::Ident(name, _) => {
                    let ty = self.var_types.get(name).ok_or_else(|| {
                        codegen_error(
                            "sizeof(expr): tipo da variavel desconhecido no lowering",
                            Some("sizeof"),
                        )
                    })?;
                    Ok(Operand::Const(ConstValue::Int(type_size(ty)?)))
                }
                _ => Err(codegen_error(
                    "sizeof(expr) so e suportado para identificadores simples neste backend",
                    Some("sizeof"),
                )),
            },
        }
    }

    pub fn lower_stmt(&mut self, stmt: &Stmt) -> LowerResult<()> {
        self.lower_stmt_with_control(stmt, ControlLabels::default())
    }

    fn lower_stmt_with_control(&mut self, stmt: &Stmt, control: ControlLabels) -> LowerResult<()> {
        match stmt {
            Stmt::Block(stmts, _) => {
                for stmt in stmts {
                    self.lower_stmt_with_control(stmt, control)?;
                }
                Ok(())
            }
            Stmt::If(cond, then_branch, else_branch, _) => {
                let cond = self.lower_expr(cond)?;
                let then_label = self.labels.fresh();
                let else_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label,
                    else_label,
                });

                self.instrs.push(TacInstr::Label(then_label));
                self.lower_stmt_with_control(then_branch, control)?;
                self.emit_jump_unless_terminated(end_label);

                self.instrs.push(TacInstr::Label(else_label));
                if let Some(else_branch) = else_branch {
                    self.lower_stmt_with_control(else_branch, control)?;
                }
                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::While(cond, body, _) => {
                let cond_label = self.labels.fresh();
                let body_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::Label(cond_label));
                let cond = self.lower_expr(cond)?;
                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label: body_label,
                    else_label: end_label,
                });

                self.instrs.push(TacInstr::Label(body_label));
                self.lower_stmt_with_control(
                    body,
                    ControlLabels {
                        break_label: Some(end_label),
                        continue_label: Some(cond_label),
                    },
                )?;
                self.emit_jump_unless_terminated(cond_label);

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::For(init, cond, inc, body, _) => {
                if let Some(init) = init {
                    self.lower_stmt_with_control(init, control)?;
                }

                let cond_label = self.labels.fresh();
                let body_label = self.labels.fresh();
                let inc_label = inc.as_ref().map(|_| self.labels.fresh());
                let end_label = self.labels.fresh();
                let continue_label = inc_label.unwrap_or(cond_label);

                self.instrs.push(TacInstr::Label(cond_label));
                if let Some(cond) = cond {
                    let cond = self.lower_expr(cond)?;
                    self.instrs.push(TacInstr::CondJump {
                        cond,
                        then_label: body_label,
                        else_label: end_label,
                    });
                }

                self.instrs.push(TacInstr::Label(body_label));
                self.lower_stmt_with_control(
                    body,
                    ControlLabels {
                        break_label: Some(end_label),
                        continue_label: Some(continue_label),
                    },
                )?;

                if let Some(inc_label) = inc_label {
                    self.instrs.push(TacInstr::Label(inc_label));
                    if let Some(inc) = inc {
                        self.lower_expr(inc)?;
                    }
                }
                self.emit_jump_unless_terminated(cond_label);

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::DoWhile(cond, body, _) => {
                let body_label = self.labels.fresh();
                let cond_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::Label(body_label));
                self.lower_stmt_with_control(
                    body,
                    ControlLabels {
                        break_label: Some(end_label),
                        continue_label: Some(cond_label),
                    },
                )?;

                self.instrs.push(TacInstr::Label(cond_label));
                let cond = self.lower_expr(cond)?;
                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label: body_label,
                    else_label: end_label,
                });

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::Break(_) => {
                let label = control.break_label.ok_or_else(|| {
                    codegen_error("break fora de loop/switch nao suportado", Some("break"))
                })?;
                self.instrs.push(TacInstr::Jump { label });
                Ok(())
            }
            Stmt::Continue(_) => {
                let label = control.continue_label.ok_or_else(|| {
                    codegen_error("continue fora de loop nao suportado", Some("continue"))
                })?;
                self.instrs.push(TacInstr::Jump { label });
                Ok(())
            }
            Stmt::ExprStmt(expr, _) => {
                self.lower_expr(expr)?;
                Ok(())
            }
            Stmt::Return(expr, _) => {
                let val = expr
                    .as_ref()
                    .map(|expr| self.lower_expr(expr))
                    .transpose()?;
                self.instrs.push(TacInstr::Return { val });
                Ok(())
            }
            Stmt::VarDecl(qty, name, init, _) => {
                self.declare_var_type(name, &qty.ty);
                if let Some(init) = init {
                    let src = self.lower_expr(init)?;
                    self.emit_copy(Operand::Var(name.clone()), src)?;
                }
                Ok(())
            }
            Stmt::Switch(disc, cases, _) => {
                let disc_op = self.lower_expr(disc)?;
                let end_label = self.labels.fresh();

                // Um label por `case`/`default`; serve tanto de alvo da
                // comparacao quanto de entrada do corpo daquele caso.
                let case_labels: Vec<LabelId> = cases.iter().map(|_| self.labels.fresh()).collect();
                let default_index = cases
                    .iter()
                    .position(|case| matches!(case.label, SwitchLabel::Default));

                // Cadeia de comparacoes: testa cada `case` (na ordem em que
                // aparece); `default` nao entra na comparacao, e usado como
                // fallback ao final da cadeia.
                for (index, case) in cases.iter().enumerate() {
                    if let SwitchLabel::Case(case_expr) = &case.label {
                        let case_val = self.lower_expr(case_expr)?;
                        let cmp = self.fresh_temp();
                        self.instrs.push(TacInstr::BinOp {
                            dst: cmp,
                            op: BinOp::Eq,
                            lhs: disc_op.clone(),
                            rhs: case_val,
                        });
                        let next_test = self.labels.fresh();
                        self.instrs.push(TacInstr::CondJump {
                            cond: Operand::Temp(cmp),
                            then_label: case_labels[index],
                            else_label: next_test,
                        });
                        self.instrs.push(TacInstr::Label(next_test));
                    }
                }
                let fallback_label = default_index.map_or(end_label, |i| case_labels[i]);
                self.instrs.push(TacInstr::Jump {
                    label: fallback_label,
                });

                // Corpo dos casos, em ordem, sem break implicito entre eles
                // (fallthrough real de C); `break` salta para `end_label`.
                for (index, case) in cases.iter().enumerate() {
                    self.instrs.push(TacInstr::Label(case_labels[index]));
                    for stmt in &case.stmts {
                        self.lower_stmt_with_control(
                            stmt,
                            ControlLabels {
                                break_label: Some(end_label),
                                continue_label: control.continue_label,
                            },
                        )?;
                    }
                }

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
        }
    }

    pub fn finish(self) -> Vec<TacInstr> {
        self.instrs
    }

    fn fresh_temp(&mut self) -> TempId {
        self.temps.fresh()
    }

    fn lower_prefix(&mut self, op: &PrefixOp, target: &Expr) -> LowerResult<Operand> {
        let dst = self.lower_assignment_target(target)?;
        let temp = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: temp,
            op: prefix_bin_op(op),
            lhs: dst.clone(),
            rhs: Operand::Const(ConstValue::Int(1)),
        });
        self.emit_copy(dst.clone(), Operand::Temp(temp))?;
        Ok(dst)
    }

    fn lower_postfix(&mut self, op: &PostfixOp, target: &Expr) -> LowerResult<Operand> {
        let dst = self.lower_assignment_target(target)?;
        let old = self.fresh_temp();
        self.emit_copy(Operand::Temp(old), dst.clone())?;

        let new = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: new,
            op: postfix_bin_op(op),
            lhs: dst.clone(),
            rhs: Operand::Const(ConstValue::Int(1)),
        });
        self.emit_copy(dst, Operand::Temp(new))?;
        Ok(Operand::Temp(old))
    }

    fn lower_assignment_target(&mut self, expr: &Expr) -> LowerResult<Operand> {
        match expr {
            Expr::Ident(name, _) => {
                // Atribuicao direta entre structs (`q = p;`) copiaria o
                // valor inteiro da struct; este backend so move 8 bytes por
                // `Copy` (todo o resto do codegen trata cada valor como um
                // unico quadword). Para structs que cabem em 8 bytes isso
                // ja funciona corretamente de gracas; para maiores,
                // copiaria so os 8 primeiros bytes silenciosamente —
                // recusa explicitamente em vez disso.
                if let Some(ty) = self.var_types.get(name) {
                    if let Type::Struct(struct_name) = resolve_alias(ty, &self.typedefs) {
                        let size = self
                            .struct_layouts
                            .get(&struct_name)
                            .map(|l| l.size)
                            .unwrap_or(8);
                        if size > 8 {
                            return Err(codegen_error(
                                "atribuicao direta entre structs maiores que 8 bytes nao suportada neste backend (copie campo a campo)",
                                Some("assign"),
                            ));
                        }
                    }
                }
                Ok(Operand::Var(name.clone()))
            }
            // `*p` como destino (`*p = x;`, `*p += 1;`, `(*p)++` etc.): o
            // ponteiro em si e um rvalue comum, mas o destino da escrita e o
            // endereco para o qual ele aponta.
            Expr::Unary(UnOp::Deref, inner, _) => {
                let ptr = self.lower_expr(inner)?;
                Ok(Operand::Deref(Box::new(ptr)))
            }
            // `arr[i] = x;` (com `arr` ponteiro): mesmo enderecamento usado
            // na leitura, via `lower_index_address`.
            Expr::Index(arr, idx, _) => {
                let addr = self.lower_index_address(arr, idx)?;
                Ok(Operand::Deref(Box::new(addr)))
            }
            // `obj.campo = x;` / `obj->campo = x;`: mesmo enderecamento
            // usado na leitura, via `lower_member_address`.
            Expr::Member(obj, access, field, _) => {
                let addr = self.lower_member_address(obj, access, field)?;
                Ok(Operand::Deref(Box::new(addr)))
            }
            _ => Err(codegen_error(
                "destino de atribuicao nao suportado no lowering",
                Some("assign"),
            )),
        }
    }

    fn emit_copy(&mut self, dst: Operand, src: Operand) -> LowerResult<()> {
        match dst {
            Operand::Temp(_) | Operand::Var(_) | Operand::Deref(_) => {
                self.instrs.push(TacInstr::Copy { dst, src });
                Ok(())
            }
            Operand::Const(_) => Err(codegen_error(
                "constante nao pode ser destino de copia",
                Some("copy"),
            )),
        }
    }

    fn emit_jump_unless_terminated(&mut self, label: LabelId) {
        if !matches!(
            self.instrs.last(),
            Some(TacInstr::Jump { .. } | TacInstr::Return { .. })
        ) {
            self.instrs.push(TacInstr::Jump { label });
        }
    }
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn lower_function(decl: &Decl) -> LowerResult<TacFunction> {
    lower_function_with_context(decl, &HashMap::new(), &HashMap::new())
}

fn lower_function_with_context(
    decl: &Decl,
    struct_layouts: &HashMap<String, StructLayout>,
    typedefs: &HashMap<String, Type>,
) -> LowerResult<TacFunction> {
    match decl {
        Decl::Function(_, name, params, body, _) => {
            let mut lowerer = Lowerer::with_context(struct_layouts.clone(), typedefs.clone());
            for (qty, param_name) in params {
                lowerer.declare_var_type(param_name, &qty.ty);
            }
            for stmt in body {
                lowerer.lower_stmt(stmt)?;
            }

            let var_sizes = lowerer.compute_var_sizes();
            Ok(TacFunction {
                name: name.clone(),
                params: params.iter().map(|(_, name)| name.clone()).collect(),
                instrs: lowerer.finish(),
                var_sizes,
            })
        }
        _ => Err(codegen_error(
            "lower_function espera Decl::Function",
            Some("lower_function"),
        )),
    }
}

pub fn lower_program(prog: &Program) -> LowerResult<TacProgram> {
    let typedefs = build_typedefs(prog);
    let struct_layouts = build_struct_layouts(prog, &typedefs);

    let mut functions = Vec::new();
    for decl in &prog.decls {
        if matches!(decl, Decl::Function(..)) {
            functions.push(lower_function_with_context(
                decl,
                &struct_layouts,
                &typedefs,
            )?);
        }
    }

    Ok(TacProgram { functions })
}

/// Segue a cadeia de `Type::Alias` ate um tipo concreto, usando a tabela de
/// `typedef` do programa. Limita a profundidade para nao travar em alias
/// ciclico malformado; nesse caso, devolve o alias original sem resolver.
fn resolve_alias(ty: &Type, typedefs: &HashMap<String, Type>) -> Type {
    let mut current = ty.clone();
    for _ in 0..8 {
        match current {
            Type::Alias(name) => match typedefs.get(&name) {
                Some(next) => current = next.clone(),
                None => return Type::Alias(name),
            },
            Type::Pointer(inner) => {
                return Type::Pointer(Box::new(resolve_alias(&inner, typedefs)))
            }
            Type::Array(inner, size) => {
                return Type::Array(Box::new(resolve_alias(&inner, typedefs)), size)
            }
            Type::Function(ret, params) => {
                let ret = QualifierType {
                    ty: resolve_alias(&ret.ty, typedefs),
                    is_const: ret.is_const,
                    is_unsigned: ret.is_unsigned,
                };
                let params = params
                    .iter()
                    .map(|param| QualifierType {
                        ty: resolve_alias(&param.ty, typedefs),
                        is_const: param.is_const,
                        is_unsigned: param.is_unsigned,
                    })
                    .collect();
                return Type::Function(Box::new(ret), params);
            }
            other => return other,
        }
    }
    current
}

/// Coleta `nome -> tipo subjacente` de todo `Decl::Typedef` do programa.
fn build_typedefs(prog: &Program) -> HashMap<String, Type> {
    let mut map = HashMap::new();
    for decl in &prog.decls {
        if let Decl::Typedef(qty, name, _) = decl {
            map.insert(name.clone(), qty.ty.clone());
        }
    }
    map
}

/// Calcula o layout de cada `Decl::StructDecl` do programa. Structs cujo
/// layout nao pode ser calculado (campo agregado aninhado — struct/array
/// dentro de struct, ainda nao suportado) sao simplesmente omitidas: usos de
/// `Expr::Member` sobre elas falham depois, no lowering, com um erro
/// explicito em vez de um layout incorreto.
fn build_struct_layouts(
    prog: &Program,
    typedefs: &HashMap<String, Type>,
) -> HashMap<String, StructLayout> {
    let mut layouts = HashMap::new();
    for decl in &prog.decls {
        if let Decl::StructDecl(name, fields, _) = decl {
            if let Ok(layout) = compute_struct_layout(fields, typedefs) {
                layouts.insert(name.clone(), layout);
            }
        }
    }
    layouts
}

fn compute_struct_layout(
    fields: &[(QualifierType, String)],
    typedefs: &HashMap<String, Type>,
) -> LowerResult<StructLayout> {
    let mut offset = 0i64;
    let mut max_align = 1i64;
    let mut laid_out = Vec::with_capacity(fields.len());

    for (qty, field_name) in fields {
        let resolved = resolve_alias(&qty.ty, typedefs);
        let size = type_size(&resolved).map_err(|_| {
            codegen_error(
                "campo de struct com tipo agregado (struct/array) aninhado nao suportado neste backend",
                Some("struct"),
            )
        })?;

        offset = align_up(offset, size);
        laid_out.push((field_name.clone(), offset, resolved));
        offset += size;
        max_align = max_align.max(size);
    }

    let total = align_up(offset, max_align.max(1)).max(1);
    Ok(StructLayout {
        fields: laid_out,
        size: total,
    })
}

fn align_up(value: i64, alignment: i64) -> i64 {
    if alignment <= 0 {
        return value;
    }
    (value + alignment - 1) / alignment * alignment
}

/// Gera o TAC e aplica todas as otimizações básicas (constant folding,
/// constant propagation e dead code elimination) até ponto fixo.
///
/// Este é o ponto de entrada recomendado para a pipeline de compilação.
pub fn lower_and_optimize(prog: &Program) -> LowerResult<TacProgram> {
    use crate::codegen::inter::optimizations::optimize_function;

    let mut tac = lower_program(prog)?;
    for func in &mut tac.functions {
        optimize_function(&mut func.instrs);
    }
    Ok(tac)
}

fn lower_literal(value: &Literal) -> ConstValue {
    match value {
        Literal::Int(value) => ConstValue::Int(*value),
        Literal::Double(value) => ConstValue::Double(*value),
        Literal::Char(value) => ConstValue::Char(*value),
        Literal::String(value) => ConstValue::String(value.clone()),
    }
}

fn prefix_bin_op(op: &PrefixOp) -> BinOp {
    match op {
        PrefixOp::Inc => BinOp::Add,
        PrefixOp::Dec => BinOp::Sub,
    }
}

fn postfix_bin_op(op: &PostfixOp) -> BinOp {
    match op {
        PostfixOp::Inc => BinOp::Add,
        PostfixOp::Dec => BinOp::Sub,
    }
}

fn type_size(ty: &Type) -> LowerResult<i64> {
    match ty {
        Type::Char => Ok(1),
        Type::Short => Ok(2),
        Type::Int | Type::Float | Type::Enum(_) => Ok(4),
        Type::Long | Type::Double | Type::Pointer(_) => Ok(8),
        Type::Array(inner, Some(len)) => {
            let elem_size = type_size(inner)?;
            Ok(elem_size * (*len as i64))
        }
        Type::Array(_, None) => Err(codegen_error(
            "lowering de sizeof(array) requer tamanho de array conhecido",
            Some("sizeof"),
        )),
        Type::Void | Type::Struct(_) | Type::Alias(_) | Type::Function(_, _) => Err(codegen_error(
            "lowering de sizeof(type) requer layout/tamanho completo",
            Some("sizeof"),
        )),
    }
}

fn codegen_error(message: &str, instruction: Option<&str>) -> CodegenError {
    CodegenError {
        message: message.to_string(),
        instruction: instruction.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        ast::ast::{QualifierType, Type},
        errors::error_data::Span,
    };

    fn span() -> Span {
        Span {
            line: 1,
            end_line: 1,
            column_start: 1,
            column_end: 1,
        }
    }

    fn int_ty() -> QualifierType {
        QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        }
    }

    fn array_ty(inner: Type, size: Option<usize>) -> QualifierType {
        QualifierType {
            ty: Type::Array(Box::new(inner), size),
            is_const: false,
            is_unsigned: false,
        }
    }

    fn int(value: i64) -> Expr {
        Expr::Literal(Literal::Int(value), span())
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident(name.to_string(), span())
    }

    #[test]
    fn lower_binary_expr_produces_temp() {
        let expr = Expr::Binary(Box::new(int(2)), BinOp::Add, Box::new(int(3)), span());
        let mut lowerer = Lowerer::new();

        let result = lowerer.lower_expr(&expr).unwrap();

        assert_eq!(result, Operand::Temp(TempId(0)));
        assert_eq!(
            lowerer.finish(),
            vec![TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Add,
                lhs: Operand::Const(ConstValue::Int(2)),
                rhs: Operand::Const(ConstValue::Int(3)),
            }]
        );
    }

    #[test]
    fn lower_if_else_produces_labels() {
        let stmt = Stmt::If(
            int(1),
            Box::new(Stmt::VarDecl(
                int_ty(),
                "x".to_string(),
                Some(int(2)),
                span(),
            )),
            Some(Box::new(Stmt::VarDecl(
                int_ty(),
                "y".to_string(),
                Some(int(3)),
                span(),
            ))),
            span(),
        );
        let mut lowerer = Lowerer::new();

        lowerer.lower_stmt(&stmt).unwrap();
        let instrs = lowerer.finish();

        assert!(matches!(
            instrs[0],
            TacInstr::CondJump {
                then_label: LabelId(0),
                else_label: LabelId(1),
                ..
            }
        ));
        assert_eq!(instrs[1], TacInstr::Label(LabelId(0)));
        assert_eq!(
            instrs[2],
            TacInstr::Copy {
                dst: Operand::Var("x".to_string()),
                src: Operand::Const(ConstValue::Int(2)),
            }
        );
        assert_eq!(instrs[3], TacInstr::Jump { label: LabelId(2) });
        assert_eq!(instrs[4], TacInstr::Label(LabelId(1)));
        assert_eq!(
            instrs[5],
            TacInstr::Copy {
                dst: Operand::Var("y".to_string()),
                src: Operand::Const(ConstValue::Int(3)),
            }
        );
        assert_eq!(instrs[6], TacInstr::Label(LabelId(2)));
    }

    #[test]
    fn lower_while_produces_backedge() {
        let stmt = Stmt::While(
            ident("keep_going"),
            Box::new(Stmt::VarDecl(
                int_ty(),
                "x".to_string(),
                Some(int(1)),
                span(),
            )),
            span(),
        );
        let mut lowerer = Lowerer::new();

        lowerer.lower_stmt(&stmt).unwrap();
        let instrs = lowerer.finish();

        assert_eq!(instrs[0], TacInstr::Label(LabelId(0)));
        assert!(matches!(
            instrs[1],
            TacInstr::CondJump {
                then_label: LabelId(1),
                else_label: LabelId(2),
                ..
            }
        ));
        assert_eq!(instrs[2], TacInstr::Label(LabelId(1)));
        assert_eq!(instrs[4], TacInstr::Jump { label: LabelId(0) });
        assert_eq!(instrs[5], TacInstr::Label(LabelId(2)));
    }

    #[test]
    fn lower_function_call_passes_args() {
        let arg0 = Expr::Binary(Box::new(int(1)), BinOp::Add, Box::new(int(2)), span());
        let expr = Expr::Call(Box::new(ident("sum")), vec![arg0, int(3)], span());
        let mut lowerer = Lowerer::new();

        let result = lowerer.lower_expr(&expr).unwrap();
        let instrs = lowerer.finish();

        assert_eq!(result, Operand::Temp(TempId(1)));
        assert_eq!(
            instrs[1],
            TacInstr::Call {
                dst: Some(TempId(1)),
                fn_name: "sum".to_string(),
                args: vec![Operand::Temp(TempId(0)), Operand::Const(ConstValue::Int(3))],
            }
        );
    }

    #[test]
    fn lower_nested_expr_correct_temp_order() {
        let rhs = Expr::Binary(Box::new(int(3)), BinOp::Mul, Box::new(int(4)), span());
        let expr = Expr::Binary(Box::new(int(2)), BinOp::Add, Box::new(rhs), span());
        let mut lowerer = Lowerer::new();

        let result = lowerer.lower_expr(&expr).unwrap();

        assert_eq!(result, Operand::Temp(TempId(1)));
        assert_eq!(
            lowerer.finish(),
            vec![
                TacInstr::BinOp {
                    dst: TempId(0),
                    op: BinOp::Mul,
                    lhs: Operand::Const(ConstValue::Int(3)),
                    rhs: Operand::Const(ConstValue::Int(4)),
                },
                TacInstr::BinOp {
                    dst: TempId(1),
                    op: BinOp::Add,
                    lhs: Operand::Const(ConstValue::Int(2)),
                    rhs: Operand::Temp(TempId(0)),
                },
            ]
        );
    }

    #[test]
    fn lower_function_keeps_name_params_and_body() {
        let decl = Decl::Function(
            int_ty(),
            "main".to_string(),
            vec![(int_ty(), "argc".to_string())],
            vec![Stmt::Return(Some(ident("argc")), span())],
            span(),
        );

        let func = lower_function(&decl).unwrap();

        assert_eq!(func.name, "main");
        assert_eq!(func.params, vec!["argc"]);
        assert_eq!(
            func.instrs,
            vec![TacInstr::Return {
                val: Some(Operand::Var("argc".to_string()))
            }]
        );
    }

    #[test]
    fn lower_fixed_array_local_populates_var_sizes() {
        let decl = Decl::Function(
            int_ty(),
            "main".to_string(),
            vec![],
            vec![
                Stmt::VarDecl(
                    array_ty(Type::Int, Some(3)),
                    "arr".to_string(),
                    None,
                    span(),
                ),
                Stmt::Return(Some(int(0)), span()),
            ],
            span(),
        );

        let func = lower_function(&decl).unwrap();

        assert_eq!(func.var_sizes.get("arr"), Some(&12));
    }

    #[test]
    fn lower_fixed_array_index_uses_address_of_base() {
        let mut lowerer = Lowerer::new();
        lowerer.declare_var_type("arr", &Type::Array(Box::new(Type::Int), Some(3)));
        let expr = Expr::Assign(
            Box::new(Expr::Index(
                Box::new(ident("arr")),
                Box::new(int(1)),
                span(),
            )),
            Box::new(int(7)),
            span(),
        );

        lowerer.lower_expr(&expr).unwrap();
        let instrs = lowerer.finish();

        assert!(instrs.iter().any(|instr| matches!(
            instr,
            TacInstr::UnOp {
                op: UnOp::AddrOf,
                src: Operand::Var(name),
                ..
            } if name == "arr"
        )));
        assert!(instrs.iter().any(|instr| matches!(
            instr,
            TacInstr::BinOp {
                op: BinOp::Mul,
                rhs: Operand::Const(ConstValue::Int(4)),
                ..
            }
        )));
        assert!(instrs.iter().any(|instr| matches!(
            instr,
            TacInstr::Copy {
                dst: Operand::Deref(_),
                ..
            }
        )));
    }
}
