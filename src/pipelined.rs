use crate::{
    cpu::{Cpu, CpuState, ExecResult},
    inst::ArchReg,
    mem::Memory,
    program::Program,
    regs::RegSet,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Pipelined {
    pub regs: RegSet,
    pub mem: Memory,
    pub prog: Program,
    pub ip: u32,
    pub cycles: u64,
}

impl Cpu for Pipelined {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        assert!(regs.get(&ArchReg::Zero).is_none());

        Self {
            regs: RegSet::new(regs),
            ip: 0,
            cycles: 0,
            mem,
            prog,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        while CpuState::Running == self.exec_one() {
            #[cfg(debug_assertions)]
            if std::env::var("VERBOSE").is_ok() {
                dbg!(&self.regs);
            }
        }

        ExecResult {
            mem: self.mem,
            cycles_taken: self.cycles,
        }
    }
}

impl Pipelined {
    fn exec_one(&mut self) -> CpuState {
        todo!()
    }
}
