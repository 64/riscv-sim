use std::fmt;

use crate::{mem::MainMemory, program::Program, regs::RegSet};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CpuState {
    Running,
    Stopped,
}

#[derive(Clone)]
pub struct ExecResult {
    pub mem: MainMemory,
    pub regs: RegSet,
    pub cycles_taken: u64,
    pub insts_retired: u64,
}

impl fmt::Debug for ExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult")
            .field("regs", &self.regs)
            .finish()
    }
}

pub trait Cpu {
    fn new(prog: Program, in_regs: RegSet, in_mem: MainMemory) -> Self;

    fn exec_all(self) -> ExecResult;
}
