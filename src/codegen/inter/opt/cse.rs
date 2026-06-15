use super::OptPass;
use crate::codegen::inter::Cfg;

pub struct CsePass;

impl OptPass for CsePass {
    fn name(&self) -> &'static str {
        "common-subexpression-elimination"
    }

    fn run(&self, _cfg: &mut Cfg) -> bool {
        false
    }
}
