use std::fmt;

use crate::common::ast::ast::Type;
use crate::common::ast::expr::{BinOp, UnOp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(pub u32);

#[derive(Debug, Clone)]
pub struct TempGen {
    next: u32,
}

impl TempGen {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn fresh(&mut self) -> TempId {
        let temp = TempId(self.next);
        self.next += 1;
        temp
    }
}

impl Default for TempGen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(pub u32);

#[derive(Debug, Clone)]
pub struct LabelGen {
    next: u32,
}

impl LabelGen {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn fresh(&mut self) -> LabelId {
        let label = LabelId(self.next);
        self.next += 1;
        label
    }
}

impl Default for LabelGen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i64),
    Double(f64),
    Char(char),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Temp(TempId),
    Var(String),
    Const(ConstValue),
    /// Endereco indireto: o ponteiro guardado em `Operand` interno e lido (ou
    /// escrito, quando usado como destino de `Copy`) atraves de deref, ex.:
    /// `*p` como destino de `*p = x;` ou como valor em `*p + 1`.
    Deref(Box<Operand>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TacInstr {
    BinOp {
        dst: TempId,
        op: BinOp,
        lhs: Operand,
        rhs: Operand,
        ty: Type,
    },
    UnOp {
        dst: TempId,
        op: UnOp,
        src: Operand,
        ty: Type,
    },
    Copy {
        dst: Operand,
        src: Operand,
        ty: Type,
    },
    Jump {
        label: LabelId,
    },
    CondJump {
        cond: Operand,
        then_label: LabelId,
        else_label: LabelId,
    },
    Call {
        dst: Option<TempId>,
        fn_name: String,
        args: Vec<Operand>,
    },
    Return {
        val: Option<Operand>,
        ty: Option<Type>,
    },
    Label(LabelId),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TacFunction {
    pub name: String,
    pub params: Vec<String>,
    pub instrs: Vec<TacInstr>,
    /// Tamanho em bytes (ja arredondado para multiplo de 8) de variaveis
    /// cujo valor nao cabe num slot escalar de 8 bytes — hoje, apenas
    /// structs locais. Variaveis ausentes daqui usam o tamanho padrao (8
    /// bytes) no codegen.
    pub var_sizes: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TacProgram {
    pub functions: Vec<TacFunction>,
}

impl fmt::Display for TempId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl fmt::Display for LabelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}", self.0)
    }
}

impl fmt::Display for ConstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstValue::Int(value) => write!(f, "{value}"),
            ConstValue::Double(value) => write!(f, "{value}"),
            ConstValue::Char(value) => write!(f, "{value:?}"),
            ConstValue::String(value) => write!(f, "{value:?}"),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Temp(temp) => write!(f, "{temp}"),
            Operand::Var(name) => write!(f, "{name}"),
            Operand::Const(value) => write!(f, "{value}"),
            Operand::Deref(inner) => write!(f, "*{inner}"),
        }
    }
}

impl fmt::Display for TacInstr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TacInstr::BinOp {
                dst,
                op,
                lhs,
                rhs,
                ty: _,
            } => {
                write!(f, "{dst} = {lhs} {} {rhs}", bin_op_symbol(op))
            }
            TacInstr::UnOp {
                dst,
                op,
                src,
                ty: _,
            } => {
                write!(f, "{dst} = {}{src}", un_op_symbol(op))
            }
            TacInstr::Copy { dst, src, ty: _ } => write!(f, "{dst} = {src}"),
            TacInstr::Jump { label } => write!(f, "goto {label}"),
            TacInstr::CondJump {
                cond,
                then_label,
                else_label,
            } => write!(f, "if {cond} goto {then_label} else goto {else_label}"),
            TacInstr::Call { dst, fn_name, args } => {
                if let Some(dst) = dst {
                    write!(f, "{dst} = ")?;
                }

                write!(f, "call {fn_name}(")?;
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
            TacInstr::Return { val, ty: _ } => {
                if let Some(val) = val {
                    write!(f, "return {val}")
                } else {
                    write!(f, "return")
                }
            }
            TacInstr::Label(label) => write!(f, "{label}:"),
        }
    }
}

fn bin_op_symbol(op: &BinOp) -> &'static str {
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

fn un_op_symbol(op: &UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
        UnOp::Deref => "*",
        UnOp::AddrOf => "&",
    }
}
