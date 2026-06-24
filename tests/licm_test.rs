use crusty::codegen::inter::opt::{LoopInvariantCodeMotionPass, OptPass};
use crusty::codegen::inter::{BasicBlock, BinaryOp, Cfg, Instruction, Value};

#[test]
fn test_licm_integration_basic() {
    let mut cfg = Cfg::new();

    // Block 0: entry
    let mut entry = BasicBlock::new("entry");
    entry.instructions.push(Instruction::Assign {
        dst: "x".to_string(),
        value: Value::Int(10),
    });
    entry.successors.push(1);
    cfg.add_block(entry);

    // Block 1: loop header
    let mut header = BasicBlock::new("header");
    header.instructions.push(Instruction::Binary {
        dst: "t1".to_string(),
        op: BinaryOp::Add,
        lhs: Value::Int(2),
        rhs: Value::Int(3),
    });
    header.instructions.push(Instruction::Binary {
        dst: "t2".to_string(),
        op: BinaryOp::Add,
        lhs: Value::Temp("t1".to_string()),
        rhs: Value::Temp("x".to_string()),
    });
    header.instructions.push(Instruction::Binary {
        dst: "t3".to_string(),
        op: BinaryOp::Add,
        lhs: Value::Temp("t2".to_string()),
        rhs: Value::Temp("y".to_string()),
    });
    header.successors.push(2);
    header.successors.push(3);
    cfg.add_block(header);

    // Block 2: loop body
    let mut body = BasicBlock::new("body");
    body.instructions.push(Instruction::Assign {
        dst: "y".to_string(),
        value: Value::Int(5),
    });
    body.successors.push(1);
    cfg.add_block(body);

    // Block 3: exit
    let mut exit = BasicBlock::new("exit");
    exit.instructions.push(Instruction::Assign {
        dst: "res".to_string(),
        value: Value::Temp("y".to_string()),
    });
    cfg.add_block(exit);

    let pass = LoopInvariantCodeMotionPass;
    let mutated = pass.run(&mut cfg);

    assert!(mutated);
    assert_eq!(cfg.blocks.len(), 5);

    // Preheader should be at index 1
    assert_eq!(cfg.blocks[1].label, "header_preheader");
    assert_eq!(cfg.blocks[1].instructions.len(), 2);
    assert_eq!(cfg.blocks[1].successors, vec![2]); // points to header (now index 2)

    // Entry block should point to preheader (index 1)
    assert_eq!(cfg.blocks[0].successors, vec![1]);

    // Loop body should point to header (index 2)
    assert_eq!(cfg.blocks[3].successors, vec![2]);
}

#[test]
fn test_licm_loop_with_invariant() {
    let mut cfg = Cfg::new();

    // Block 0: entry
    let mut entry = BasicBlock::new("entry");
    entry.instructions.push(Instruction::Assign {
        dst: "a".to_string(),
        value: Value::Int(5),
    });
    entry.instructions.push(Instruction::Assign {
        dst: "b".to_string(),
        value: Value::Int(10),
    });
    entry.instructions.push(Instruction::Assign {
        dst: "i".to_string(),
        value: Value::Int(0),
    });
    entry.instructions.push(Instruction::Assign {
        dst: "result".to_string(),
        value: Value::Int(0),
    });
    entry.successors.push(1);
    cfg.add_block(entry);

    // Block 1: loop header
    let mut header = BasicBlock::new("header");
    header.instructions.push(Instruction::Binary {
        dst: "t1".to_string(),
        op: BinaryOp::Add,
        lhs: Value::Temp("a".to_string()),
        rhs: Value::Temp("b".to_string()),
    });
    header.instructions.push(Instruction::Binary {
        dst: "t2".to_string(),
        op: BinaryOp::Mul,
        lhs: Value::Temp("i".to_string()),
        rhs: Value::Temp("t1".to_string()),
    });
    header.instructions.push(Instruction::Binary {
        dst: "result".to_string(),
        op: BinaryOp::Add,
        lhs: Value::Temp("result".to_string()),
        rhs: Value::Temp("t2".to_string()),
    });
    header.instructions.push(Instruction::Binary {
        dst: "i".to_string(),
        op: BinaryOp::Add,
        lhs: Value::Temp("i".to_string()),
        rhs: Value::Int(1),
    });
    header.successors.push(2);
    header.successors.push(3);
    cfg.add_block(header);

    // Block 2: loop body/latch
    let mut body = BasicBlock::new("body");
    body.instructions.push(Instruction::Nop);
    body.successors.push(1);
    cfg.add_block(body);

    // Block 3: exit
    let mut exit = BasicBlock::new("exit");
    exit.instructions.push(Instruction::Assign {
        dst: "res".to_string(),
        value: Value::Temp("result".to_string()),
    });
    cfg.add_block(exit);

    let pass = LoopInvariantCodeMotionPass;
    let mutated = pass.run(&mut cfg);

    assert!(mutated);
    assert_eq!(cfg.blocks.len(), 5);

    // Preheader should contain "t1 = a + b"
    assert_eq!(cfg.blocks[1].label, "header_preheader");
    assert_eq!(cfg.blocks[1].instructions.len(), 1);
    assert_eq!(
        cfg.blocks[1].instructions[0],
        Instruction::Binary {
            dst: "t1".to_string(),
            op: BinaryOp::Add,
            lhs: Value::Temp("a".to_string()),
            rhs: Value::Temp("b".to_string()),
        }
    );

    // Header block (index 2) should no longer contain "t1 = a + b"
    assert_eq!(cfg.blocks[2].instructions.len(), 3);
    assert_eq!(
        cfg.blocks[2].instructions[0],
        Instruction::Binary {
            dst: "t2".to_string(),
            op: BinaryOp::Mul,
            lhs: Value::Temp("i".to_string()),
            rhs: Value::Temp("t1".to_string()),
        }
    );
}
