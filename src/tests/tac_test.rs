use crate::common::ast::ast::Type;
use crate::common::ast::expr::BinOp;
use crate::ir::tac::{LabelGen, LabelId, Operand, TacInstr, TempGen, TempId};

#[test]
fn tac_instr_display_binop() {
    let instr = TacInstr::BinOp {
        dst: TempId(0),
        op: BinOp::Add,
        lhs: Operand::Temp(TempId(1)),
        rhs: Operand::Temp(TempId(2)),
        ty: Type::Int,
    };

    assert_eq!(instr.to_string(), "t0 = t1 + t2");
}

#[test]
fn temp_gen_increments() {
    let mut gen = TempGen::new();

    assert_eq!(gen.fresh(), TempId(0));
    assert_eq!(gen.fresh(), TempId(1));
}

#[test]
fn label_gen_unique() {
    let mut gen = LabelGen::new();

    assert_eq!(gen.fresh(), LabelId(0));
    assert_eq!(gen.fresh(), LabelId(1));
}
