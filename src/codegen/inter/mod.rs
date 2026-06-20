pub mod opt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub blocks: Vec<BasicBlock>,
}

impl Cfg {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn add_block(&mut self, block: BasicBlock) {
        self.blocks.push(block);
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<Instruction>,
    pub successors: Vec<usize>,
}

impl BasicBlock {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            instructions: Vec::new(),
            successors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Assign {
        dst: String,
        value: Value,
    },
    Binary {
        dst: String,
        op: BinaryOp,
        lhs: Value,
        rhs: Value,
    },
    Nop,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Int(i64),
    Temp(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}
