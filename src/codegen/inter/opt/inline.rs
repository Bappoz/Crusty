use super::OptPass;
use crate::codegen::inter::Cfg;

pub struct InliningPass;

impl OptPass for InliningPass {
    fn name(&self) -> &'static str {
        "inlining"
    }

    fn run(&self, _cfg: &mut Cfg) -> bool {
        false
    }
}
