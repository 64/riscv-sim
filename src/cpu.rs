use crate::{inst::ArchReg, mem::Memory, program::Program};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CpuState {
    Running,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub mem: Memory,
    pub cycles_taken: u64,
    pub insts_retired: u64,
}

pub trait Cpu {
    fn new(prog: Program, in_regs: HashMap<ArchReg, u32>, in_mem: Memory) -> Self;

    fn exec_all(self) -> ExecResult;
}
