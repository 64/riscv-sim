use crate::{
    cpu::{Cpu, CpuState, ExecResult},
    inst::{ArchReg, Inst},
    mem::Memory,
    program::Program,
    regs::RegSet,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Emulated {
    regs: RegSet,
    mem: Memory,
    prog: Program,
    pc: u32,
    cycles: u64,
}

impl Cpu for Emulated {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        assert!(regs.get(&ArchReg::Zero).is_none());

        Self {
            regs: RegSet::new(regs),
            pc: 0,
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
            insts_retired: self.cycles,
        }
    }
}

impl Emulated {
    fn exec_one(&mut self) -> CpuState {
        let next_inst = match self.prog.fetch(self.pc) {
            Some(i) => i,
            None => return CpuState::Stopped,
        };

        let mut advance_pc = true;

        match *next_inst {
            Inst::LoadByte(dst, src) => {
                let val = self.mem.readb(self.regs.ref_to_addr(src));
                self.regs.set(dst, val);
            }
            Inst::LoadHalfWord(dst, src) => {
                let val = self.mem.readh(self.regs.ref_to_addr(src));
                self.regs.set(dst, val);
            }
            Inst::LoadWord(dst, src) => {
                let val = self.mem.readw(self.regs.ref_to_addr(src));
                self.regs.set(dst, val);
            }
            Inst::StoreByte(src, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeb(dst, self.regs.get(src));
            }
            Inst::StoreHalfWord(src, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeh(dst, self.regs.get(src));
            }
            Inst::StoreWord(src, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writew(dst, self.regs.get(src));
            }
            Inst::Add(dst, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                self.regs.set(dst, a.wrapping_add(b));
            }
            Inst::AddImm(dst, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                self.regs.set(dst, a.wrapping_add(b));
            }
            Inst::ShiftLeftLogicalImm(dst, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                self.regs.set(dst, a.wrapping_shl(b));
            }
            Inst::JumpAndLink(dst, ref offset) => {
                self.regs.set(dst, self.pc + 1);
                self.pc += offset.0;
                advance_pc = false;
            }
            Inst::BranchIfEqual(src0, src1, ref dst) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a == b {
                    self.pc = self.prog.labels[dst];
                    advance_pc = false;
                }
            }
            Inst::BranchIfNotEqual(src0, src1, ref dst) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a != b {
                    self.pc = self.prog.labels[dst];
                    advance_pc = false;
                }
            }
            Inst::BranchIfGreaterEqual(src0, src1, ref dst) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a >= b {
                    self.pc = self.prog.labels[dst];
                    advance_pc = false;
                }
            }
            Inst::Halt => unreachable!(),
        }

        if advance_pc {
            self.pc += 1;
        }

        self.cycles += 1;

        CpuState::Running
    }
}
