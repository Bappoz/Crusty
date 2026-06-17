use super::OptPass;
use crate::codegen::inter::Cfg;

pub struct LoopInvariantCodeMotionPass;

impl OptPass for LoopInvariantCodeMotionPass {
    fn name(&self) -> &'static str {
        "loop-invariant-code-motion"
    }

    fn run(&self, _cfg: &mut Cfg) -> bool {
        false
    }
}
