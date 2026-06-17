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
}

#[derive(Debug, Clone, PartialEq)]
pub enum TacInstr {
    BinOp {
        dst: TempId,
        op: BinOp,
        lhs: Operand,
        rhs: Operand,
    },
    UnOp {
        dst: TempId,
        op: UnOp,
        src: Operand,
    },
    Copy {
        dst: TempId,
        src: Operand,
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
    },
    Label(LabelId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TacFunction {
    pub name: String,
    pub params: Vec<String>,
    pub instrs: Vec<TacInstr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TacProgram {
    pub functions: Vec<TacFunction>,
}
