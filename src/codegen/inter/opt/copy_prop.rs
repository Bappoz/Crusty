use super::OptPass;
use crate::codegen::inter::Cfg;

pub struct CopyPropagationPass;

impl OptPass for CopyPropagationPass {
    fn name(&self) -> &'static str {
        "copy-propagation"
    }

    fn run(&self, _cfg: &mut Cfg) -> bool {
        false
    }
}
